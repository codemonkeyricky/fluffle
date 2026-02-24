mod file_ops;
mod bash_exec;
pub use file_ops::FileOpsPlugin;
pub use bash_exec::BashExecPlugin;

pub fn register_plugins() -> Vec<Box<dyn crate::plugin::Plugin>> {
    let mut plugins: Vec<Box<dyn crate::plugin::Plugin>> = Vec::new();

    plugins.push(Box::new(FileOpsPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(BashExecPlugin) as Box<dyn crate::plugin::Plugin>);

    plugins
}