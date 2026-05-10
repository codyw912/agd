use crate::paths::AgdPaths;
use crate::project::Project;
use crate::workspace;
use anyhow::Result;

pub fn create(paths: &AgdPaths, project: &mut Project, name: &str) -> Result<()> {
    let path = workspace::ensure_workspace(paths, project, name)?;
    println!("Created workspace {name}");
    println!("  {}", path.display());
    Ok(())
}

pub fn list(project: &Project) {
    for workspace in &project.workspaces {
        println!(
            "{}\t{}\t{}",
            workspace.id,
            workspace.status,
            workspace.path.display()
        );
    }
}
