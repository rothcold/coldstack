use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn sanitize_path(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn workspace_path(task_id: &str) -> PathBuf {
    PathBuf::from("agent_workspaces").join(sanitize_path(task_id))
}

fn workspace_metadata_path(workspace: &Path) -> PathBuf {
    workspace.join(".coldstack-source")
}

pub fn slugify_branch_label(value: &str) -> String {
    let mut slug = value
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    slug.trim_matches('-').to_string()
}

pub fn default_branch_name(title: &str, description: &str) -> String {
    let label = if !title.trim().is_empty() {
        title
    } else {
        description
    };
    let slug = slugify_branch_label(label);
    if slug.is_empty() {
        "task/work-item".to_string()
    } else {
        format!("task/{}", slug)
    }
}

pub async fn ensure_workspace(
    task_id: &str,
    source: &str,
    source_branch: &str,
    branch: &str,
) -> Result<PathBuf, String> {
    let workspace = workspace_path(task_id);
    let expected_source = normalize_source(source)?;
    let expected_branch = source_branch.trim().to_string();

    if workspace.join(".git").exists() {
        if workspace_needs_reclone(&workspace, &expected_source, &expected_branch).await? {
            std::fs::remove_dir_all(&workspace)
                .map_err(|e| format!("Failed to recreate workspace {:?}: {}", workspace, e))?;
        } else {
            ensure_branch_checked_out(&workspace, branch).await?;
            return Ok(workspace);
        }
    }

    if workspace.exists() {
        let mut entries = std::fs::read_dir(&workspace)
            .map_err(|e| format!("Failed to inspect workspace {:?}: {}", workspace, e))?;
        if entries.next().transpose().map_err(|e| {
            format!("Failed to inspect workspace {:?}: {}", workspace, e)
        })?.is_some()
        {
            return Err(format!(
                "Workspace {:?} already exists and is not a git repository",
                workspace
            ));
        }
    } else if let Some(parent) = workspace.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create workspace parent {:?}: {}", parent, e))?;
    }

    clone_into_workspace(source, source_branch, &workspace).await?;
    write_workspace_metadata(&workspace, &expected_source, &expected_branch)?;
    configure_git_identity(&workspace).await?;
    ensure_branch_checked_out(&workspace, branch).await?;
    Ok(workspace)
}

fn normalize_source(source: &str) -> Result<String, String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("Task source is required".to_string());
    }

    if let Some(local_path) = resolve_local_source(trimmed) {
        let canonical = std::fs::canonicalize(&local_path).map_err(|e| {
            format!(
                "Failed to resolve task source {}: {}",
                local_path.display(),
                e
            )
        })?;
        return Ok(canonical.to_string_lossy().to_string());
    }

    Ok(trimmed.to_string())
}

fn parse_workspace_metadata(contents: &str) -> Option<(String, String)> {
    let mut lines = contents.lines();
    let source = lines.next()?.trim().to_string();
    let branch = lines.next()?.trim().to_string();
    if source.is_empty() || branch.is_empty() {
        return None;
    }
    Some((source, branch))
}

fn read_workspace_metadata(workspace: &Path) -> Option<(String, String)> {
    let contents = std::fs::read_to_string(workspace_metadata_path(workspace)).ok()?;
    parse_workspace_metadata(&contents)
}

fn write_workspace_metadata(
    workspace: &Path,
    source: &str,
    source_branch: &str,
) -> Result<(), String> {
    std::fs::write(
        workspace_metadata_path(workspace),
        format!("{source}\n{source_branch}\n"),
    )
    .map_err(|e| format!("Failed to write workspace metadata {:?}: {}", workspace, e))
}

async fn workspace_needs_reclone(
    workspace: &Path,
    expected_source: &str,
    expected_branch: &str,
) -> Result<bool, String> {
    let (current_source, current_branch) = match read_workspace_metadata(workspace) {
        Some(metadata) => metadata,
        None => (
            current_workspace_source(workspace).await?,
            current_workspace_source_branch(workspace).await?,
        ),
    };

    Ok(current_source != expected_source || current_branch != expected_branch)
}

async fn current_workspace_source(workspace: &Path) -> Result<String, String> {
    let remote = run_git_capture(workspace, &["config", "--get", "remote.origin.url"]).await?;
    normalize_source(remote.trim())
}

async fn current_workspace_source_branch(workspace: &Path) -> Result<String, String> {
    let refs = run_git_capture(
        workspace,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/remotes/origin",
        ],
    )
    .await?;

    let branch = refs
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "origin/HEAD")
        .find_map(|line| line.strip_prefix("origin/"))
        .ok_or_else(|| format!("Failed to determine source branch for workspace {:?}", workspace))?;

    Ok(branch.to_string())
}

pub async fn finalize_workspace(
    task_id: &str,
    branch: &str,
    title: &str,
) -> Result<Option<String>, String> {
    let workspace = workspace_path(task_id);

    if !workspace.join(".git").exists() {
        return Err(format!(
            "Task workspace {:?} is not a git repository",
            workspace
        ));
    }

    configure_git_identity(&workspace).await?;
    ensure_branch_checked_out(&workspace, branch).await?;

    if !has_changes(&workspace).await? {
        return Ok(None);
    }

    run_git(&workspace, &["add", "-A"]).await?;
    run_git(
        &workspace,
        &["commit", "-m", &format!("task({}): {}", task_id, title)],
    )
    .await?;

    let commit = run_git_capture(&workspace, &["rev-parse", "HEAD"]).await?;
    Ok(Some(commit.trim().to_string()))
}

fn resolve_local_source(source: &str) -> Option<PathBuf> {
    let trimmed = source.trim();
    if let Some(stripped) = trimmed.strip_prefix("file://") {
        return Some(PathBuf::from(stripped));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() || trimmed.starts_with("./") || trimmed.starts_with("../") {
        return Some(path.to_path_buf());
    }

    None
}

pub async fn publish_workspace(task_id: &str, branch: &str) -> Result<(), String> {
    let workspace = workspace_path(task_id);

    if !workspace.join(".git").exists() {
        return Err(format!(
            "Task workspace {:?} is not a git repository",
            workspace
        ));
    }

    ensure_branch_checked_out(&workspace, branch).await?;
    run_git(&workspace, &["push", "-u", "origin", branch]).await
}

async fn clone_into_workspace(
    source: &str,
    source_branch: &str,
    workspace: &Path,
) -> Result<(), String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("Task source is required".to_string());
    }
    let branch = source_branch.trim();
    if branch.is_empty() {
        return Err("Task source branch is required".to_string());
    }

    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .arg("clone")
        .arg("--quiet")
        .arg("--no-hardlinks")
        .arg("--branch")
        .arg(branch)
        .arg("--single-branch");

    if let Some(local_path) = resolve_local_source(trimmed) {
        if !local_path.exists() {
            return Err(format!("Local git source does not exist: {}", local_path.display()));
        }
        cmd.arg(local_path);
    } else {
        cmd.arg("--depth").arg("1").arg(trimmed);
    }

    cmd.arg(workspace);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run git clone: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "unknown git clone error".to_string()
    };
    Err(format!("Failed to clone task source: {}", detail))
}

async fn configure_git_identity(workspace: &Path) -> Result<(), String> {
    ensure_git_config(workspace, "user.name", "Coldstack Agent").await?;
    ensure_git_config(workspace, "user.email", "coldstack-agent@local").await?;
    Ok(())
}

async fn ensure_git_config(workspace: &Path, key: &str, value: &str) -> Result<(), String> {
    let existing = run_git_capture(workspace, &["config", "--get", key]).await;
    if existing.is_ok() {
        return Ok(());
    }

    run_git(workspace, &["config", key, value]).await
}

async fn ensure_branch_checked_out(workspace: &Path, branch: &str) -> Result<(), String> {
    let exists = run_git(workspace, &["rev-parse", "--verify", branch]).await.is_ok();
    if exists {
        return run_git(workspace, &["checkout", branch]).await;
    }

    run_git(workspace, &["checkout", "-b", branch]).await
}

async fn has_changes(workspace: &Path) -> Result<bool, String> {
    let output = run_git_capture(workspace, &["status", "--short"]).await?;
    Ok(!output.trim().is_empty())
}

async fn run_git(workspace: &Path, args: &[&str]) -> Result<(), String> {
    let _ = run_git_capture(workspace, args).await?;
    Ok(())
}

async fn run_git_capture(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(workspace)
        .arg("-c")
        .arg("commit.gpgsign=false")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to run git {:?}: {}", args, e))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "unknown git error".to_string()
    };
    Err(format!("git {:?} failed: {}", args, detail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_resolve_local_source_for_paths() {
        assert_eq!(
            resolve_local_source("/tmp/repo").unwrap(),
            PathBuf::from("/tmp/repo")
        );
        assert_eq!(
            resolve_local_source("./repo").unwrap(),
            PathBuf::from("./repo")
        );
        assert_eq!(
            resolve_local_source("file:///tmp/repo").unwrap(),
            PathBuf::from("/tmp/repo")
        );
        assert!(resolve_local_source("https://github.com/openai/openai.git").is_none());
        assert!(resolve_local_source("git@github.com:openai/openai.git").is_none());
    }

    #[test]
    fn test_default_branch_name_uses_human_readable_slug() {
        assert_eq!(
            default_branch_name("Build Weather Forecast Website", ""),
            "task/build-weather-forecast-website"
        );
        assert_eq!(default_branch_name("", "Add mobile layout"), "task/add-mobile-layout");
    }

    #[tokio::test]
    async fn test_ensure_workspace_clones_local_repo() {
        let repo_dir = std::env::temp_dir().join(format!("coldstack-source-{}", Uuid::new_v4()));
        let task_id = format!("T-{}", Uuid::new_v4());
        let workspace = workspace_path(&task_id);
        let branch = "task/build-weather-forecast-website";

        std::fs::create_dir_all(&repo_dir).unwrap();
        run_git(Path::new("."), &["init", "--quiet", repo_dir.to_str().unwrap()])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.name", "Test User"])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.email", "test@example.com"])
            .await
            .unwrap();
        std::fs::write(repo_dir.join("README.md"), "# fixture\n").unwrap();
        run_git(&repo_dir, &["add", "README.md"]).await.unwrap();
        run_git(&repo_dir, &["commit", "-m", "init"]).await.unwrap();

        let prepared = ensure_workspace(&task_id, repo_dir.to_str().unwrap(), "main", branch)
            .await
            .unwrap();
        assert_eq!(prepared, workspace);
        assert!(prepared.join(".git").exists());
        let current_branch = run_git_capture(&prepared, &["branch", "--show-current"])
            .await
            .unwrap();
        assert_eq!(current_branch.trim(), branch);

        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&prepared);
    }

    #[tokio::test]
    async fn test_ensure_workspace_reclones_when_source_changes() {
        let repo_one = std::env::temp_dir().join(format!("coldstack-source-a-{}", Uuid::new_v4()));
        let repo_two = std::env::temp_dir().join(format!("coldstack-source-b-{}", Uuid::new_v4()));
        let task_id = format!("T-{}", Uuid::new_v4());
        let workspace = workspace_path(&task_id);
        let branch = "task/source-switch";

        for (repo_dir, contents) in [(&repo_one, "first\n"), (&repo_two, "second\n")] {
            std::fs::create_dir_all(repo_dir).unwrap();
            run_git(Path::new("."), &["init", "--quiet", repo_dir.to_str().unwrap()])
                .await
                .unwrap();
            run_git(repo_dir, &["config", "user.name", "Test User"])
                .await
                .unwrap();
            run_git(repo_dir, &["config", "user.email", "test@example.com"])
                .await
                .unwrap();
            std::fs::write(repo_dir.join("README.md"), contents).unwrap();
            run_git(repo_dir, &["add", "README.md"]).await.unwrap();
            run_git(repo_dir, &["commit", "-m", "init"]).await.unwrap();
        }

        ensure_workspace(&task_id, repo_one.to_str().unwrap(), "main", branch)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(workspace.join("README.md")).unwrap(),
            "first\n"
        );

        ensure_workspace(&task_id, repo_two.to_str().unwrap(), "main", branch)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(workspace.join("README.md")).unwrap(),
            "second\n"
        );
        assert_eq!(
            current_workspace_source(&workspace).await.unwrap(),
            std::fs::canonicalize(&repo_two)
                .unwrap()
                .to_string_lossy()
                .to_string()
        );

        let _ = std::fs::remove_dir_all(&repo_one);
        let _ = std::fs::remove_dir_all(&repo_two);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[tokio::test]
    async fn test_ensure_workspace_reclones_when_source_branch_changes() {
        let repo_dir = std::env::temp_dir().join(format!("coldstack-source-{}", Uuid::new_v4()));
        let task_id = format!("T-{}", Uuid::new_v4());
        let workspace = workspace_path(&task_id);
        let branch = "task/branch-switch";

        std::fs::create_dir_all(&repo_dir).unwrap();
        run_git(Path::new("."), &["init", "--quiet", repo_dir.to_str().unwrap()])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.name", "Test User"])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.email", "test@example.com"])
            .await
            .unwrap();
        std::fs::write(repo_dir.join("README.md"), "main\n").unwrap();
        run_git(&repo_dir, &["add", "README.md"]).await.unwrap();
        run_git(&repo_dir, &["commit", "-m", "main"]).await.unwrap();
        run_git(&repo_dir, &["checkout", "-b", "develop"]).await.unwrap();
        std::fs::write(repo_dir.join("README.md"), "develop\n").unwrap();
        run_git(&repo_dir, &["add", "README.md"]).await.unwrap();
        run_git(&repo_dir, &["commit", "-m", "develop"]).await.unwrap();
        run_git(&repo_dir, &["checkout", "main"]).await.unwrap();

        ensure_workspace(&task_id, repo_dir.to_str().unwrap(), "main", branch)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(workspace.join("README.md")).unwrap(),
            "main\n"
        );

        ensure_workspace(&task_id, repo_dir.to_str().unwrap(), "develop", branch)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(workspace.join("README.md")).unwrap(),
            "develop\n"
        );
        assert_eq!(
            current_workspace_source_branch(&workspace).await.unwrap(),
            "develop"
        );

        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[tokio::test]
    async fn test_finalize_workspace_commits_without_touching_source_repo() {
        let repo_dir = std::env::temp_dir().join(format!("coldstack-source-{}", Uuid::new_v4()));
        let task_id = format!("T-{}", Uuid::new_v4());
        let workspace = workspace_path(&task_id);
        let branch = "task/implement-feature";

        run_git(Path::new("."), &["init", "--quiet", repo_dir.to_str().unwrap()])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.name", "Test User"])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.email", "test@example.com"])
            .await
            .unwrap();
        std::fs::write(repo_dir.join("README.md"), "# fixture\n").unwrap();
        run_git(&repo_dir, &["add", "README.md"]).await.unwrap();
        run_git(&repo_dir, &["commit", "-m", "init"]).await.unwrap();

        ensure_workspace(&task_id, repo_dir.to_str().unwrap(), "main", branch)
            .await
            .unwrap();
        std::fs::write(workspace.join("feature.txt"), "done\n").unwrap();

        let commit = finalize_workspace(&task_id, branch, "Implement feature")
            .await
            .unwrap();
        assert!(commit.is_some());

        let remote_branch = run_git_capture(
            &repo_dir,
            &["show-ref", "--verify", &format!("refs/heads/{}", branch)],
        )
        .await;
        assert!(remote_branch.is_err());

        let local_commit = run_git_capture(&workspace, &["rev-parse", "HEAD"])
            .await
            .unwrap();
        assert_eq!(commit.unwrap(), local_commit.trim());

        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[tokio::test]
    async fn test_publish_workspace_pushes_task_branch_to_source_repo() {
        let repo_dir = std::env::temp_dir().join(format!("coldstack-source-{}", Uuid::new_v4()));
        let task_id = format!("T-{}", Uuid::new_v4());
        let workspace = workspace_path(&task_id);
        let branch = "task/publish-feature";

        run_git(Path::new("."), &["init", "--quiet", repo_dir.to_str().unwrap()])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.name", "Test User"])
            .await
            .unwrap();
        run_git(&repo_dir, &["config", "user.email", "test@example.com"])
            .await
            .unwrap();
        std::fs::write(repo_dir.join("README.md"), "# fixture\n").unwrap();
        run_git(&repo_dir, &["add", "README.md"]).await.unwrap();
        run_git(&repo_dir, &["commit", "-m", "init"]).await.unwrap();

        ensure_workspace(&task_id, repo_dir.to_str().unwrap(), "main", branch)
            .await
            .unwrap();
        std::fs::write(workspace.join("feature.txt"), "done\n").unwrap();
        finalize_workspace(&task_id, branch, "Publish feature")
            .await
            .unwrap();
        publish_workspace(&task_id, branch).await.unwrap();

        let remote_branch = run_git_capture(
            &repo_dir,
            &["show-ref", "--verify", &format!("refs/heads/{}", branch)],
        )
        .await
        .unwrap();
        assert!(!remote_branch.trim().is_empty());

        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&workspace);
    }
}
