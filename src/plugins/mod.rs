mod bash_exec;
mod explore;
mod file_ops;
mod git_ops;
mod task;
pub use bash_exec::BashExecPlugin;
pub use explore::ExplorePlugin;
pub use file_ops::FileOpsPlugin;
pub use git_ops::GitOpsPlugin;
pub use task::TaskPlugin;

pub fn register_plugins() -> Vec<Box<dyn crate::plugin::Plugin>> {
    let mut plugins: Vec<Box<dyn crate::plugin::Plugin>> = Vec::new();

    plugins.push(Box::new(FileOpsPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(BashExecPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(GitOpsPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(TaskPlugin) as Box<dyn crate::plugin::Plugin>);
    plugins.push(Box::new(ExplorePlugin) as Box<dyn crate::plugin::Plugin>);

    plugins
}
