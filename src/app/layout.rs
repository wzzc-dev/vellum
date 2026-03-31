use super::*;

pub(super) fn next_untitled_path(root: &Path) -> PathBuf {
    let mut index = 1usize;
    loop {
        let candidate = if index == 1 {
            root.join("untitled.md")
        } else {
            root.join(format!("untitled-{index}.md"))
        };
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}
