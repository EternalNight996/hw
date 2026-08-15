use std::collections::HashMap;
use std::sync::RwLock;

use e_utils::once_cell::sync::Lazy;

use super::TestMode;

/// 测试模式注册表
pub struct Registry {
  modes: HashMap<&'static str, &'static dyn TestMode>,
}

impl Default for Registry {
  fn default() -> Self {
    Self { modes: HashMap::new() }
  }
}

impl Registry {
  /// 注册一个模式（后注册的同名模式覆盖先注册的）
  pub fn register(&mut self, mode: &'static dyn TestMode) {
    self.modes.insert(mode.name(), mode);
  }
  /// 按名字取模式
  pub fn get(&self, name: &str) -> Option<&'static dyn TestMode> {
    self.modes.get(name).copied()
  }
  /// 所有已注册模式（名字, 描述），按名字排序
  pub fn list(&self) -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<_> = self.modes.iter().map(|(k, m)| (*k, m.description())).collect();
    v.sort_unstable();
    v
  }
}

static REGISTRY: Lazy<RwLock<Registry>> = Lazy::new(|| RwLock::new(Registry::default()));

/// 注册一个模式
pub fn register(mode: &'static dyn TestMode) {
  if let Ok(mut r) = REGISTRY.write() {
    r.register(mode);
  }
}

/// 按名字取模式
pub fn get(name: &str) -> Option<&'static dyn TestMode> {
  REGISTRY.read().ok().and_then(|r| r.get(name))
}

/// 所有已注册模式（名字, 描述）
pub fn list() -> Vec<(&'static str, &'static str)> {
  REGISTRY.read().map(|r| r.list()).unwrap_or_default()
}

/// 返回注册表只读引用（供 runner 使用）
pub fn registered_modes() -> std::sync::RwLockReadGuard<'static, Registry> {
  REGISTRY.read().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
pub(crate) fn clear_for_test() {
  if let Ok(mut r) = REGISTRY.write() {
    r.modes.clear();
  }
}