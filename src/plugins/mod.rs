mod file_ops;
mod bash_exec;
mod git_ops;
pub use file_ops::FileOpsPlugin;
pub use bash_exec::BashExecPlugin;
pub use git_ops::GitOpsPlugin;

pub fn register_plugins() -> Vec<Box<dyn crate::plugin::Plugin>> {
    let mut plugins: Vec<Box<dyn crate::plugin::Plugin>> = Vec::new();

    plugins.push(Box::new(FileOpsPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(BashExecPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(GitOpsPlugin) as Box<dyn crate::plugin::Plugin>);

    plugins
}