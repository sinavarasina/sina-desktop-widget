//! Foreground application booster combining cgroup v2 `dmem.low` VRAM protection
//! and systemd `SetUnitProperties` (`CPUWeight` / `MemoryLow`) via `waywi::dbus`.

use std::fs;
use std::path::{Path, PathBuf};

use waywi::prelude::*;
use waywi::system::MemorySnapshot;
use waywi::system::os::ProcessCgroupInfo;

const SYSTEMD_BUS_NAME: &str = "org.freedesktop.systemd1";
const SYSTEMD_OBJECT_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_MANAGER_IFACE: &str = "org.freedesktop.systemd1.Manager";
const SYSTEMD_SET_UNIT_PROPS: &str = "SetUnitProperties";

const DEFAULT_CGROUP_ROOT: &str = "/sys/fs/cgroup";
const BOOST_CPU_WEIGHT: u64 = 200;
const NORMAL_CPU_WEIGHT: u64 = 100;
/// Protect 90% of VRAM capacity for the focused app scope, leaving 10% headroom for compositor/bar.
const DMEM_LOW_NUMERATOR: u64 = 9;
const DMEM_LOW_DENOMINATOR: u64 = 10;
/// Conservative 25% RAM soft protection floor for the focused app scope.
const MEMORY_LOW_DIVISOR: u64 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmemRegion {
    pub name: String,
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveBoost {
    pub unit: String,
    pub cgroup_dir: PathBuf,
    pub dmem_regions: Vec<String>,
    pub dmem_active: bool,
    pub systemd_active: bool,
}

/// Manages ephemeral foreground boost state for the currently focused Hyprland client.
///
/// Automatically restores `dmem.low` to `0` on `Drop` so exiting or restarting the widget
/// never leaves stale VRAM protection behind.
#[derive(Debug)]
pub struct ForegroundBooster {
    cgroup_root: PathBuf,
    cached_regions: Option<Vec<DmemRegion>>,
    pub active: Option<ActiveBoost>,
}

impl Default for ForegroundBooster {
    fn default() -> Self {
        Self::with_cgroup_root(PathBuf::from(DEFAULT_CGROUP_ROOT))
    }
}

impl Drop for ForegroundBooster {
    fn drop(&mut self) {
        if let Some(prev) = self.active.take()
            && prev.dmem_active
        {
            restore_dmem_low(&prev.cgroup_dir, &prev.dmem_regions);
        }
    }
}

impl ForegroundBooster {
    #[must_use]
    pub fn with_cgroup_root(cgroup_root: PathBuf) -> Self {
        Self {
            cgroup_root,
            cached_regions: None,
            active: None,
        }
    }

    /// Updates foreground boost to match the newly focused window's cgroup (or clears boost when
    /// focus moves to desktop or an ineligible scope such as `session-*.scope`).
    pub fn on_focus_change(
        &mut self,
        cgroup: Option<&ProcessCgroupInfo>,
        memory: Option<&MemorySnapshot>,
        ctx: &EffectCtx<'_, crate::app::msg::Msg>,
    ) {
        let Some(info) = cgroup.filter(|c| is_boostable_cgroup(c)) else {
            self.clear_active(ctx);
            return;
        };
        let Some(unit) = info.systemd_unit.as_deref() else {
            self.clear_active(ctx);
            return;
        };

        let unit_rel = resolve_unit_cgroup_path(&info.cgroup_path, unit);
        let cgroup_dir = self.resolve_cgroup_dir(&unit_rel, &info.cgroup_path);

        if let Some(cur) = &self.active
            && cur.unit == unit
            && cur.cgroup_dir == cgroup_dir
        {
            return;
        }

        self.clear_active(ctx);

        let regions = self.dmem_regions().to_vec();
        let (dmem_active, applied_regions) = apply_dmem_low(&cgroup_dir, &regions);

        let mem_low_bytes = memory.map_or(0, |m| m.total.as_bytes() / MEMORY_LOW_DIVISOR);
        let systemd_active = enqueue_systemd_unit_props(ctx, unit, BOOST_CPU_WEIGHT, mem_low_bytes);

        if dmem_active || systemd_active {
            self.active = Some(ActiveBoost {
                unit: unit.to_owned(),
                cgroup_dir,
                dmem_regions: applied_regions,
                dmem_active,
                systemd_active,
            });
        }
    }

    pub fn clear_active(&mut self, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
        let Some(prev) = self.active.take() else {
            return;
        };
        if prev.dmem_active {
            restore_dmem_low(&prev.cgroup_dir, &prev.dmem_regions);
        }
        if prev.systemd_active {
            let _ = enqueue_systemd_unit_props(ctx, &prev.unit, NORMAL_CPU_WEIGHT, 0);
        }
    }

    #[must_use]
    pub fn short_badge(&self) -> Option<&'static str> {
        let active = self.active.as_ref()?;
        match (active.dmem_active, active.systemd_active) {
            (true, true) => Some("VRAM+CPU"),
            (true, false) => Some("VRAM+"),
            (false, true) => Some("Boost"),
            (false, false) => None,
        }
    }

    #[must_use]
    pub fn tooltip_line(&self, cgroup: Option<&ProcessCgroupInfo>) -> Option<String> {
        if let Some(active) = &self.active {
            let detail = match (active.dmem_active, active.systemd_active) {
                (true, true) => "dmem + systemd",
                (true, false) => "dmem",
                (false, true) => "systemd",
                (false, false) => return None,
            };
            return Some(format!("Boost: active ({detail})"));
        }
        let info = cgroup?;
        if !info.unified_v2 {
            return Some(String::from("Boost: skipped (non-cgroupv2)"));
        }
        if !info.cgroup_path.contains("/app.slice/") {
            let unit = info.systemd_unit.as_deref().unwrap_or(&info.cgroup_path);
            return Some(format!("Boost: skipped ({unit})"));
        }
        None
    }

    fn dmem_regions(&mut self) -> &[DmemRegion] {
        let root = &self.cgroup_root;
        self.cached_regions.get_or_insert_with(|| {
            fs::read_to_string(root.join("dmem.capacity"))
                .map(|raw| parse_dmem_capacity(&raw))
                .unwrap_or_default()
        })
    }

    fn resolve_cgroup_dir(&self, unit_rel: &str, raw_rel: &str) -> PathBuf {
        let unit_dir = join_cgroup_rel(&self.cgroup_root, unit_rel);
        if unit_dir.join("dmem.low").exists() {
            return unit_dir;
        }
        let raw_dir = join_cgroup_rel(&self.cgroup_root, raw_rel);
        if raw_dir.join("dmem.low").exists() {
            return raw_dir;
        }
        unit_dir
    }
}

/// Defensive eligibility check: only boost unified cgroup v2 scopes/services isolated inside
/// `/app.slice/`, never `session-*.scope`, system slices, or the widget process itself.
#[must_use]
pub fn is_boostable_cgroup(info: &ProcessCgroupInfo) -> bool {
    if !info.unified_v2 || info.pid == 0 || info.pid == std::process::id() {
        return false;
    }
    let path = info.cgroup_path.trim();
    if !path.starts_with('/') || path.split('/').any(|seg| seg == "..") {
        return false;
    }
    if !path.contains("/app.slice/") {
        return false;
    }
    info.systemd_unit
        .as_deref()
        .is_some_and(|u| u.ends_with(".scope") || u.ends_with(".service"))
}

/// Trims nested sub-cgroups (e.g. `.../app-flatpak-foo.scope/sandbox`) back to the owning systemd
/// unit directory so `dmem.low` targets the scope root.
#[must_use]
pub fn resolve_unit_cgroup_path(cgroup_path: &str, unit: &str) -> String {
    let mut parts = Vec::new();
    for seg in cgroup_path.split('/').filter(|s| !s.is_empty()) {
        parts.push(seg);
        if seg == unit {
            return format!("/{}", parts.join("/"));
        }
    }
    cgroup_path.to_owned()
}

/// Parses `/sys/fs/cgroup/dmem.capacity` lines (`<region> <bytes>`).
#[must_use]
pub fn parse_dmem_capacity(contents: &str) -> Vec<DmemRegion> {
    let mut out = Vec::new();
    for line in contents.lines() {
        let mut words = line.split_whitespace();
        let Some(name) = words.next() else {
            continue;
        };
        let Some(cap_str) = words.next() else {
            continue;
        };
        if words.next().is_some() {
            continue;
        }
        if let Ok(capacity_bytes) = cap_str.parse::<u64>()
            && capacity_bytes > 0
        {
            out.push(DmemRegion {
                name: name.to_owned(),
                capacity_bytes,
            });
        }
    }
    out
}

/// Builds `org.freedesktop.systemd1.Manager.SetUnitProperties` arguments (`sba(sv)`).
#[must_use]
pub fn build_set_unit_properties_args(
    unit: &str,
    cpu_weight: u64,
    memory_low_bytes: u64,
) -> Vec<DbusValue> {
    vec![
        DbusValue::String(unit.to_owned()),
        DbusValue::Bool(true),
        DbusValue::Array(vec![
            DbusValue::Struct(vec![
                DbusValue::String(String::from("CPUWeight")),
                DbusValue::variant(DbusValue::U64(cpu_weight)),
            ]),
            DbusValue::Struct(vec![
                DbusValue::String(String::from("MemoryLow")),
                DbusValue::variant(DbusValue::U64(memory_low_bytes)),
            ]),
        ]),
    ]
}

fn enqueue_systemd_unit_props(
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
    unit: &str,
    cpu_weight: u64,
    memory_low_bytes: u64,
) -> bool {
    let Ok(dbus) = ctx.dbus() else {
        return false;
    };
    let args = build_set_unit_properties_args(unit, cpu_weight, memory_low_bytes);
    dbus.call_method(
        DbusBus::Session,
        SYSTEMD_BUS_NAME,
        SYSTEMD_OBJECT_PATH,
        SYSTEMD_MANAGER_IFACE,
        SYSTEMD_SET_UNIT_PROPS,
        &args,
    )
    .is_ok()
}

fn join_cgroup_rel(root: &Path, rel: &str) -> PathBuf {
    let trimmed = rel.trim_start_matches('/');
    root.join(trimmed)
}

fn apply_dmem_low(cgroup_dir: &Path, regions: &[DmemRegion]) -> (bool, Vec<String>) {
    if regions.is_empty() {
        return (false, Vec::new());
    }
    let dmem_low = cgroup_dir.join("dmem.low");
    if !dmem_low.exists() {
        return (false, Vec::new());
    }

    let mut applied = Vec::with_capacity(regions.len());
    for region in regions {
        let target =
            region.capacity_bytes.saturating_mul(DMEM_LOW_NUMERATOR) / DMEM_LOW_DENOMINATOR;
        if target == 0 {
            continue;
        }
        let payload = format!("{} {target}\n", region.name);
        if fs::write(&dmem_low, payload).is_ok() {
            applied.push(region.name.clone());
        }
    }
    (!applied.is_empty(), applied)
}

fn restore_dmem_low(cgroup_dir: &Path, regions: &[String]) {
    let dmem_low = cgroup_dir.join("dmem.low");
    if !dmem_low.exists() {
        return;
    }
    for name in regions {
        let _ = fs::write(&dmem_low, format!("{name} 0\n"));
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use waywi::prelude::DbusValue;
    use waywi::system::os::ProcessCgroupInfo;

    use super::{
        ActiveBoost, ForegroundBooster, apply_dmem_low, build_set_unit_properties_args,
        is_boostable_cgroup, parse_dmem_capacity, resolve_unit_cgroup_path, restore_dmem_low,
    };

    #[test]
    fn parses_dmem_capacity_lines_and_ignores_invalid_entries() {
        let raw = "\
drm/0000:03:00.0/vram 12868124672
drm/0000:0c:00.0/vram 536870912
malformed
drm/zero/vram 0
extra tokens here 123
";
        let regions = parse_dmem_capacity(raw);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].name, "drm/0000:03:00.0/vram");
        assert_eq!(regions[0].capacity_bytes, 12_868_124_672);
        assert_eq!(regions[1].name, "drm/0000:0c:00.0/vram");
        assert_eq!(regions[1].capacity_bytes, 536_870_912);
    }

    #[test]
    fn enforces_app_slice_guard_and_rejects_session_scope() {
        let valid_scope = ProcessCgroupInfo {
            pid: 999_999,
            cgroup_path: "/user.slice/user-1000.slice/user@1000.service/app.slice/app-steam-1091500.scope/sandbox".into(),
            systemd_unit: Some("app-steam-1091500.scope".into()),
            unified_v2: true,
        };
        assert!(is_boostable_cgroup(&valid_scope));
        assert_eq!(
            resolve_unit_cgroup_path(&valid_scope.cgroup_path, "app-steam-1091500.scope"),
            "/user.slice/user-1000.slice/user@1000.service/app.slice/app-steam-1091500.scope"
        );

        let session_scope = ProcessCgroupInfo {
            pid: 999_998,
            cgroup_path: "/user.slice/user-1000.slice/session-1.scope".into(),
            systemd_unit: Some("session-1.scope".into()),
            unified_v2: true,
        };
        assert!(!is_boostable_cgroup(&session_scope));

        let traversal = ProcessCgroupInfo {
            pid: 999_997,
            cgroup_path: "/user.slice/app.slice/../session-1.scope".into(),
            systemd_unit: Some("session-1.scope".into()),
            unified_v2: true,
        };
        assert!(!is_boostable_cgroup(&traversal));

        let own_pid = ProcessCgroupInfo {
            pid: std::process::id(),
            cgroup_path: "/user.slice/user-1000.slice/user@1000.service/app.slice/widget.scope"
                .into(),
            systemd_unit: Some("widget.scope".into()),
            unified_v2: true,
        };
        assert!(!is_boostable_cgroup(&own_pid));
    }

    #[test]
    fn encodes_systemd_set_unit_properties_with_struct_and_variant() {
        let args = build_set_unit_properties_args("app-game.scope", 200, 4_294_967_296);
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].as_str(), Some("app-game.scope"));
        assert_eq!(args[1].as_bool(), Some(true));
        let DbusValue::Array(props) = &args[2] else {
            panic!("expected Array for a(sv)");
        };
        assert_eq!(props.len(), 2);

        let cpu_fields = props[0].as_struct().expect("CPUWeight struct");
        assert_eq!(cpu_fields[0].as_str(), Some("CPUWeight"));
        assert!(cpu_fields[1].as_variant().is_some());
        assert_eq!(cpu_fields[1].as_u64(), Some(200));

        let mem_fields = props[1].as_struct().expect("MemoryLow struct");
        assert_eq!(mem_fields[0].as_str(), Some("MemoryLow"));
        assert!(mem_fields[1].as_variant().is_some());
        assert_eq!(mem_fields[1].as_u64(), Some(4_294_967_296));
    }

    #[test]
    fn writes_dmem_low_on_boost_and_resets_to_zero_on_restore_and_drop() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sina-widget-dmem-test-{}-{stamp}",
            std::process::id()
        ));
        let scope_dir =
            root.join("user.slice/user-1000.slice/user@1000.service/app.slice/app-game.scope");
        fs::create_dir_all(&scope_dir).unwrap();
        fs::write(root.join("dmem.capacity"), "drm/0000:03:00.0/vram 10000\n").unwrap();
        let dmem_low = scope_dir.join("dmem.low");
        fs::write(&dmem_low, "drm/0000:03:00.0/vram 0\n").unwrap();

        let regions = parse_dmem_capacity(&fs::read_to_string(root.join("dmem.capacity")).unwrap());
        let (ok, applied) = apply_dmem_low(&scope_dir, &regions);
        assert!(ok);
        assert_eq!(applied, vec!["drm/0000:03:00.0/vram"]);
        assert_eq!(
            fs::read_to_string(&dmem_low).unwrap(),
            "drm/0000:03:00.0/vram 9000\n"
        );

        restore_dmem_low(&scope_dir, &applied);
        assert_eq!(
            fs::read_to_string(&dmem_low).unwrap(),
            "drm/0000:03:00.0/vram 0\n"
        );

        // Verify Drop also resets dmem.low to 0 cleanly when the widget exits.
        let (ok2, applied2) = apply_dmem_low(&scope_dir, &regions);
        assert!(ok2);
        {
            let mut booster = ForegroundBooster::with_cgroup_root(root.clone());
            booster.active = Some(ActiveBoost {
                unit: "app-game.scope".into(),
                cgroup_dir: scope_dir.clone(),
                dmem_regions: applied2,
                dmem_active: true,
                systemd_active: false,
            });
        }
        assert_eq!(
            fs::read_to_string(&dmem_low).unwrap(),
            "drm/0000:03:00.0/vram 0\n"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
