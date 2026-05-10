use crate::project::Project;

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
