mod file_ops;
pub use file_ops::FileOpsPlugin;

use inventory;

pub fn register_plugins() -> Vec<Box<dyn crate::plugin::Plugin>> {
    let mut plugins: Vec<Box<dyn crate::plugin::Plugin>> = Vec::new();

    plugins.push(Box::new(FileOpsPlugin) as Box<dyn crate::plugin::Plugin>);

    plugins
}