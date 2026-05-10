use crate::project::Project;

pub fn show(project: &Project) {
    println!("Name: {}", project.agent_identity.name);
    println!("Email: {}", project.agent_identity.email);
    println!("Signing: {}", project.agent_identity.signing);
}
