use crate::project::Project;

pub const PROTECTED_BRANCH_EXACT: &[&str] = &[
    "main",
    "master",
    "trunk",
    "develop",
    "stable",
    "production",
    "prod",
];

pub const PROTECTED_BRANCH_PREFIXES: &[&str] = &["release/", "stable/", "production/", "prod/"];

pub fn is_agent_branch(project: &Project, branch: &str) -> bool {
    !branch.is_empty() && !is_protected_branch(project, branch)
}

pub fn is_protected_branch(project: &Project, branch: &str) -> bool {
    branch.is_empty()
        || branch == project.default_target
        || PROTECTED_BRANCH_EXACT.contains(&branch)
        || PROTECTED_BRANCH_PREFIXES
            .iter()
            .any(|prefix| branch.starts_with(prefix))
}

pub fn shell_case_patterns() -> String {
    let mut patterns = PROTECTED_BRANCH_EXACT.to_vec();
    let wildcard_patterns = PROTECTED_BRANCH_PREFIXES
        .iter()
        .map(|prefix| format!("{prefix}*"));
    patterns.extend(PROTECTED_BRANCH_PREFIXES.iter().copied());
    patterns
        .into_iter()
        .map(str::to_string)
        .chain(wildcard_patterns)
        .collect::<Vec<_>>()
        .join("|")
}
