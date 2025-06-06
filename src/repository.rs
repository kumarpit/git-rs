// Definitions and methods for the gitrs "repository"
use core::panic;
use std::fs::{self, File, canonicalize, copy};
use std::io::{BufReader, Cursor, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::ZlibEncoder;

pub struct Repository {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
}

impl Repository {
    pub fn new(worktree: &Path) -> Self {
        Self {
            worktree: worktree.to_path_buf(),
            gitdir: worktree.join(".gitrs"),
        }
    }

    pub fn init(worktree: &Path) -> Self {
        let gitdir = worktree.join(".gitrs");
        if worktree.exists() {
            if !worktree.is_dir() {
                panic!("Expected a directory at: {}", worktree.display());
            }

            if gitdir.exists() && !is_empty_dir(gitdir.as_path()) {
                panic!("Expected empty directory at: {}", gitdir.display());
            }
        } else {
            fs::create_dir_all(gitdir.as_path()).unwrap_or_else(|e| {
                panic!("Failed to create the path {}: {}", gitdir.display(), e)
            });
        }

        let repository = Self::new(worktree);

        let did_create_dirs = [
            repository.repo_dir(&["branches"], true),
            repository.repo_dir(&["objects"], true),
            repository.repo_dir(&["refs", "tags"], true),
            repository.repo_dir(&["refs", "heads"], true),
        ]
        .iter()
        .all(|opt| opt.is_some());

        if !did_create_dirs {
            panic!("An error occurred when initializing the gitrs repository");
        }

        repository.write_to_repo_file(
            &repository
                .repo_file(&["description"], false)
                .expect("Could not make descrption file"),
            b"Unamed repository; edit this file 'description' to name the repository.\n",
        );

        repository.write_to_repo_file(
            &repository
                .repo_file(&["HEAD"], false)
                .expect("Could not make HEAD file"),
            b"ref: refs/heads/master\n",
        );

        // TODO: Figure out config file management

        repository
    }

    // TODO: these should return Result instead and check for file existence here
    pub fn get_path_to_file(&self, paths: &[&str]) -> Option<PathBuf> {
        let path = self.repo_file(paths, false).unwrap();
        if !path.exists() { None } else { Some(path) }
    }

    pub fn get_path_to_dir(&self, paths: &[&str]) -> Option<PathBuf> {
        self.repo_dir(paths, false)
    }

    pub fn upsert_file(&self, paths: &[&str], data: &Vec<u8>) -> Option<PathBuf> {
        let path = self.repo_file(paths, true).expect("Could not create path");
        let file = File::create(&path).expect("Could not create file");
        let mut encoder = ZlibEncoder::new(file, Compression::default());
        encoder
            .write_all(&data)
            .expect("Could not write compressed data");
        Some(path)
    }

    /// Finds the root directory of the nearest gitrs repository by traversing parents of the
    /// `current_path`
    pub fn find_repository(current_path: &Path) -> Option<Repository> {
        let canonical_current_path = canonicalize(current_path).unwrap();
        if canonical_current_path.join(".gitrs").exists() {
            Some(Repository::new(current_path))
        } else {
            match canonical_current_path.parent() {
                None => None,
                Some(parent_dir) => Repository::find_repository(parent_dir),
            }
        }
    }

    /////////////////////////////////////
    /// Repository File Management
    /////////////////////////////////////

    // Computes the path under a repository's gitrs directory
    fn repo_path(&self, paths: &[&str]) -> PathBuf {
        paths.iter().fold(self.gitdir.clone(), |mut acc, path| {
            acc.push(path);
            acc
        })
    }

    // Same as repo_path, but creates the trailing directories if they don't exist if the
    // should_mkdir flag is set
    fn repo_file(&self, paths: &[&str], should_mkdir: bool) -> Option<PathBuf> {
        match self.repo_dir(&paths[..paths.len() - 1], should_mkdir) {
            Some(_) => Some(self.repo_path(paths)),
            None => None,
        }
    }

    // Same as repo_path, but creates the path if the should_mkdir flag is true
    fn repo_dir(&self, paths: &[&str], should_mkdir: bool) -> Option<PathBuf> {
        let path = self.repo_path(paths);
        if path.exists() {
            if !path.is_dir() {
                panic!("Expected a directory at {}", path.display());
            }
            Some(path)
        } else if should_mkdir {
            fs::create_dir_all(&path)
                .unwrap_or_else(|e| panic!("Failed to create the path {}: {}", path.display(), e));
            Some(path)
        } else {
            None
        }
    }

    fn write_to_repo_file(&self, path: &PathBuf, content: &[u8]) {
        File::create(path)
            .unwrap_or_else(|e| panic!("Could not create file {}: {}", path.display(), e))
            .write_all(content)
            .unwrap_or_else(|e| panic!("Could not write to file {}: {}", path.display(), e));
    }
}

fn is_empty_dir(path: &Path) -> bool {
    path.is_dir() && fs::read_dir(path).map_or(false, |mut entries| entries.next().is_none())
}
