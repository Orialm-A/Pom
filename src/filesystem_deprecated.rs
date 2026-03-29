// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::io::Read;
//     use tempfile::tempdir;
//
//     mod subdirs_creation {
//         use super::*;
//         use std::fs::File;
//
//         #[test]
//         fn creates_dirs_when_missing() {
//             let tmp = tempfile::tempdir().unwrap();
//             let project_root = tmp.path();
//
//             let dirs = vec![
//                 project_root.join("src/app"),
//                 project_root.join("resources/doc"),
//             ];
//
//             create_directories(&dirs).unwrap();
//
//             assert!(project_root.join("src").is_dir());
//             assert!(project_root.join("src/app").is_dir());
//             assert!(project_root.join("resources/doc").is_dir());
//         }
//
//         #[test]
//         fn succeeds_if_dirs_already_exist() {
//             let tmp = tempfile::tempdir().unwrap();
//             let project_root = tmp.path();
//
//             fs::create_dir_all(project_root.join("src/app")).unwrap();
//
//             let dirs = vec![
//                 project_root.join("src/app"),
//                 project_root.join("resources/doc"),
//             ];
//
//             create_directories(&dirs).unwrap();
//
//             assert!(project_root.join("src/app").is_dir());
//             assert!(project_root.join("resources/doc").is_dir());
//         }
//
//         #[test]
//         fn fails_if_dir_path_is_blocked_by_file() {
//             let tmp = tempfile::tempdir().unwrap();
//             let project_root = tmp.path();
//
//             // Create a file "src" so "src/app" cannot become a directory
//             File::create(project_root.join("src")).unwrap();
//
//             let dirs = vec![project_root.join("src/app")];
//
//             let err = create_directories(&dirs).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemDirCreationFail);
//         }
//     }
//
//     mod validate_dir_entry_tests {
//         use super::*;
//
//         #[test]
//         fn validate_dir_entry_file_ok_relative_path_and_kind() {
//             let src = tempdir().unwrap();
//             let src_root = src.path();
//
//             let nested = src_root.join("a/b");
//             fs::create_dir_all(&nested).unwrap();
//
//             let file_path = nested.join("hello.txt");
//             fs::write(&file_path, b"hi").unwrap();
//
//             // WalkDir yields the root first, then children. We'll find our file entry.
//             let entry = walkdir::WalkDir::new(src_root)
//                 .into_iter()
//                 .find_map(|e| match e {
//                     Ok(de) if de.path() == file_path => Some(Ok(de)),
//                     Ok(_) => None,
//                     Err(err) => Some(Err(err)),
//                 })
//                 .expect("expected to find the file entry");
//
//             let validated = validate_dir_entry(entry, src_root).unwrap();
//
//             assert_eq!(validated.full_path, file_path);
//             assert_eq!(
//                 validated.relative_path,
//                 std::path::PathBuf::from("a/b/hello.txt")
//             );
//             assert!(matches!(validated.kind, EntryKind::File));
//         }
//
//         #[test]
//         fn validate_dir_entry_dir_ok_relative_path_and_kind() {
//             let src = tempdir().unwrap();
//             let src_root = src.path();
//
//             let nested = src_root.join("dir1/dir2");
//             fs::create_dir_all(&nested).unwrap();
//
//             let entry = walkdir::WalkDir::new(src_root)
//                 .into_iter()
//                 .find_map(|e| match e {
//                     Ok(de) if de.path() == nested => Some(Ok(de)),
//                     Ok(_) => None,
//                     Err(err) => Some(Err(err)),
//                 })
//                 .expect("expected to find the dir entry");
//
//             let validated = validate_dir_entry(entry, src_root).unwrap();
//
//             assert_eq!(validated.full_path, nested);
//             assert_eq!(
//                 validated.relative_path,
//                 std::path::PathBuf::from("dir1/dir2")
//             );
//             assert!(matches!(validated.kind, EntryKind::Directory));
//         }
//
//         #[test]
//         fn validate_dir_entry_err_is_mapped() {
//             let src = tempdir().unwrap();
//             let src_root = src.path();
//
//             // Force a WalkDir error by iterating a non-existent path.
//             let mut it = walkdir::WalkDir::new(src_root.join("does-not-exist")).into_iter();
//             let first = it.next().expect("expected an item (Err) from iterator");
//
//             let err = validate_dir_entry(first, src_root).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemEntryInvalid);
//             assert!(err.1.is_some());
//         }
//
//         #[test]
//         fn validate_dir_entry_strip_prefix_fail_is_mapped() {
//             let src = tempdir().unwrap();
//             let src_root = src.path();
//
//             let file_path = src_root.join("x.txt");
//             fs::write(&file_path, b"x").unwrap();
//
//             let entry = walkdir::WalkDir::new(src_root)
//                 .into_iter()
//                 .find_map(|e| match e {
//                     Ok(de) if de.path() == file_path => Some(Ok(de)),
//                     Ok(_) => None,
//                     Err(err) => Some(Err(err)),
//                 })
//                 .expect("expected to find file entry");
//
//             // Pass a different root so strip_prefix fails
//             let other_root = tempdir().unwrap();
//
//             let err = validate_dir_entry(entry, other_root.path()).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemStripPathPrefixFail);
//             assert!(err.1.is_some());
//         }
//
//         #[test]
//         fn validate_dir_entry_unsupported_entry_type_fifo() {
//             use std::ffi::CString;
//
//             let src = tempdir().unwrap();
//             let src_root = src.path();
//
//             let fifo_path = src_root.join("myfifo");
//             let cpath = CString::new(fifo_path.to_string_lossy().as_bytes()).unwrap();
//             let rc = unsafe { libc::mkfifo(cpath.as_ptr(), 0o644) };
//             assert_eq!(rc, 0, "mkfifo failed");
//
//             let entry = walkdir::WalkDir::new(src_root)
//                 .into_iter()
//                 .find_map(|e| match e {
//                     Ok(de) if de.path() == fifo_path => Some(Ok(de)),
//                     Ok(_) => None,
//                     Err(err) => Some(Err(err)),
//                 })
//                 .expect("expected to find fifo entry");
//
//             let err = validate_dir_entry(entry, src_root).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemUnsupportedEntryType);
//             assert!(err.1.unwrap().contains("myfifo"));
//         }
//     }
//
//     mod copy_files_tests {
//         use super::*;
//
//         fn read_to_string(p: &std::path::Path) -> String {
//             let mut s = String::new();
//             fs::File::open(p).unwrap().read_to_string(&mut s).unwrap();
//             s
//         }
//
//         #[test]
//         fn copy_files_copies_tree_and_counts_files() {
//             let src = tempdir().unwrap();
//             let dst = tempdir().unwrap();
//
//             let src_root = src.path();
//             let dst_root = dst.path();
//
//             fs::create_dir_all(src_root.join("a/b")).unwrap();
//             fs::write(src_root.join("root.txt"), "root").unwrap();
//             fs::write(src_root.join("a/file1.txt"), "one").unwrap();
//             fs::write(src_root.join("a/b/file2.txt"), "two").unwrap();
//
//             let n = copy_files(src_root, dst_root, ExistingFilePolicy::Overwrite, &None).unwrap();
//             assert_eq!(n, 3);
//
//             assert_eq!(read_to_string(&dst_root.join("root.txt")), "root");
//             assert_eq!(read_to_string(&dst_root.join("a/file1.txt")), "one");
//             assert_eq!(read_to_string(&dst_root.join("a/b/file2.txt")), "two");
//             assert!(dst_root.join("a/b").is_dir());
//         }
//
//         #[test]
//         fn copy_files_fails_if_source_missing() {
//             let dst = tempdir().unwrap();
//             let err = copy_files(
//                 std::path::Path::new("this-path-should-not-exist-___"),
//                 dst.path(),
//                 ExistingFilePolicy::Overwrite,
//                 &None,
//             )
//             .unwrap_err();
//
//             assert_eq!(err.0, PomErrorCode::FilesystemCopySourceMissing);
//         }
//
//         #[test]
//         fn copy_files_fails_if_source_not_dir() {
//             let src = tempdir().unwrap();
//             let dst = tempdir().unwrap();
//
//             let file = src.path().join("not_a_dir.txt");
//             fs::write(&file, "x").unwrap();
//
//             let err =
//                 copy_files(&file, dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemCopySourceNotDir);
//         }
//
//         #[test]
//         fn copy_files_fails_on_existing_file_when_policy_fail() {
//             let src = tempdir().unwrap();
//             let dst = tempdir().unwrap();
//
//             fs::write(src.path().join("a.txt"), "SRC").unwrap();
//
//             // Pre-create destination file with same relative path
//             fs::write(dst.path().join("a.txt"), "DST").unwrap();
//
//             let err =
//                 copy_files(src.path(), dst.path(), ExistingFilePolicy::Fail, &None).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemFileOverwriteForbidden);
//         }
//
//         #[test]
//         fn copy_files_overwrites_existing_file_when_policy_allows() {
//             let src = tempdir().unwrap();
//             let dst = tempdir().unwrap();
//
//             fs::write(src.path().join("a.txt"), "NEW").unwrap();
//             fs::write(dst.path().join("a.txt"), "OLD").unwrap();
//
//             let n =
//                 copy_files(src.path(), dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap();
//             assert_eq!(n, 1);
//
//             let content = read_to_string(&dst.path().join("a.txt"));
//             assert_eq!(content, "NEW");
//         }
//
//         #[test]
//         fn copy_files_skips_symlinks() {
//             use std::os::unix::fs::symlink;
//
//             let src = tempdir().unwrap();
//             let dst = tempdir().unwrap();
//
//             fs::write(src.path().join("real.txt"), "REAL").unwrap();
//             symlink(src.path().join("real.txt"), src.path().join("link.txt")).unwrap();
//
//             let n =
//                 copy_files(src.path(), dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap();
//
//             // Only real.txt copied; link.txt should be skipped
//             assert_eq!(n, 1);
//             assert!(dst.path().join("real.txt").exists());
//             assert!(!dst.path().join("link.txt").exists());
//         }
//     }
//
//     mod write_file_tests {
//         use super::*;
//
//         fn read_to_string(p: &std::path::Path) -> String {
//             let mut s = String::new();
//             fs::File::open(p).unwrap().read_to_string(&mut s).unwrap();
//             s
//         }
//
//         #[test]
//         fn write_file_creates_and_writes() {
//             let dir = tempdir().unwrap();
//             let p = dir.path().join("x.txt");
//
//             write_file(&p, "hello", &ExistingFilePolicy::Overwrite).unwrap();
//             assert_eq!(read_to_string(&p), "hello");
//         }
//
//         #[test]
//         fn write_file_overwrites_and_truncates() {
//             let dir = tempdir().unwrap();
//             let p = dir.path().join("x.txt");
//
//             fs::write(&p, "0123456789").unwrap();
//             write_file(&p, "abc", &ExistingFilePolicy::Overwrite).unwrap();
//
//             // truncate(true) should have removed old tail
//             assert_eq!(read_to_string(&p), "abc");
//         }
//
//         #[test]
//         fn write_file_fails_if_exists_and_policy_fail() {
//             let dir = tempdir().unwrap();
//             let p = dir.path().join("x.txt");
//
//             fs::write(&p, "existing").unwrap();
//             let err = write_file(&p, "new", &ExistingFilePolicy::Fail).unwrap_err();
//
//             assert_eq!(err.0, PomErrorCode::FilesystemFileOverwriteForbidden);
//         }
//     }
//
//     mod module_search_tests {
//         use super::*;
//
//         fn create_test_file_tree() -> (tempfile::TempDir, PathBuf) {
//             let temp_dir = tempdir().unwrap();
//             let project_root = temp_dir.path().join("project_root");
//
//             fs::create_dir(&project_root).unwrap();
//
//             fs::create_dir_all(project_root.join("src/services")).unwrap();
//             fs::create_dir_all(project_root.join("src/peripherals")).unwrap();
//             fs::create_dir_all(project_root.join("unit_tests")).unwrap();
//
//             fs::write(project_root.join("src/services/timer.h"), "").unwrap();
//             fs::write(project_root.join("src/services/timer.c"), "").unwrap();
//
//             fs::write(project_root.join("src/peripherals/timer.c"), "").unwrap();
//             fs::write(project_root.join("src/peripherals/tim.h"), "").unwrap();
//
//             fs::write(project_root.join("unit_tests/timer.h"), "").unwrap();
//             fs::write(project_root.join("unit_tests/tests_timer.c"), "").unwrap();
//
//             (temp_dir, project_root)
//         }
//
//         fn expected_timer_research_result() -> ModulesFound {
//             let mut expected = ModulesFound::new();
//             expected.insert_header(&PathBuf::from("src/services"));
//             expected.insert_source(&PathBuf::from("src/services"));
//             expected.insert_source(&PathBuf::from("src/peripherals"));
//             expected.insert_header(&PathBuf::from("unit_tests"));
//
//             expected
//         }
//
//         #[test]
//         fn find_all_modules_complete_or_not() {
//             let (_temp_dir, project_root) = create_test_file_tree();
//
//             let mut search_scope = HashSet::new();
//             search_scope.insert(PathBuf::from("src"));
//             search_scope.insert(PathBuf::from("unit_tests"));
//
//             let result = search_module("timer", &project_root, &search_scope).unwrap();
//
//             assert_eq!(result, expected_timer_research_result());
//         }
//
//         #[test]
//         fn count_number_of_modules_found() {
//             let search_result = expected_timer_research_result();
//             assert_eq!(search_result.get_number_of_modules(), 3);
//         }
//
//         #[test]
//         fn dont_select_unique_location_if_several_available() {
//             let search_result = expected_timer_research_result();
//             assert_eq!(
//                 search_result.get_unique_location().unwrap_err().0,
//                 PomErrorCode::FileSystemModuleSearchResultNotUnique
//             );
//         }
//
//         #[test]
//         fn correctly_get_all_locations_as_text() {
//             let search_result = expected_timer_research_result();
//             let mut locations = search_result.get_all_locations_as_text();
//             locations.sort();
//             let expected_locations: Vec<String> = Vec::from([
//                 "src/peripherals".to_string(),
//                 "src/services".to_string(),
//                 "unit_tests".to_string(),
//             ]);
//
//             assert_eq!(locations, expected_locations);
//         }
//     }
//
//     mod file_rename_tests {
//         use super::*;
//
//         fn create_temp_files() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
//             let temp_dir = tempdir().unwrap();
//             let test_root = temp_dir.path().join("project_root");
//
//             fs::create_dir(&test_root).unwrap();
//
//             let file_hello_world = test_root.join("hello_world.txt");
//             write_file(
//                 &file_hello_world,
//                 "Hello, World!",
//                 &ExistingFilePolicy::Overwrite,
//             )
//             .unwrap();
//
//             let file_hello_rust = test_root.join("hello_rust.txt");
//             write_file(
//                 &file_hello_rust,
//                 "Hello, Rust!",
//                 &ExistingFilePolicy::Overwrite,
//             )
//             .unwrap();
//
//             return (temp_dir, test_root, file_hello_world, file_hello_rust);
//         }
//
//         #[test]
//         fn reject_not_found_files() {
//             let (_temp_dir, test_root, _, file_hello_rust) = create_temp_files();
//
//             let wrong_file = test_root.join("wrong_file.txt");
//
//             let err = rename_file(&wrong_file, &file_hello_rust).unwrap_err();
//
//             assert_eq!(err.0, PomErrorCode::FilesystemRenameOriginNotFound);
//         }
//
//         #[test]
//         fn reject_existing_destination() {
//             let (_temp_dir, _, file_hello_world, file_hello_rust) = create_temp_files();
//             let err = rename_file(&file_hello_world, &file_hello_rust).unwrap_err();
//             assert_eq!(err.0, PomErrorCode::FilesystemRenameDestinationExists);
//         }
//
//         #[test]
//         fn rename_file_renames_file() {
//             let (_temp_dir, _test_root, file_hello_world, _) = create_temp_files();
//
//             let new_path = file_hello_world.with_file_name("hello_universe.txt");
//
//             rename_file(&file_hello_world, &new_path).unwrap();
//
//             assert!(!file_hello_world.exists());
//             assert!(new_path.exists());
//
//             let renamed_file_content = fs::read_to_string(&new_path).unwrap();
//
//             assert_eq!(renamed_file_content, String::from("Hello, World!"));
//         }
//     }
//
//     #[cfg(test)]
//     mod file_read_tests {
//         use super::*;
//         // use std::fs;
//         // use std::path::PathBuf;
//         use tempfile::{TempDir, tempdir};
//
//         fn create_temp_file() -> (TempDir, PathBuf) {
//             let temp_dir = tempdir().unwrap();
//             let file_path = temp_dir.path().join("hello.txt");
//
//             write_file(&file_path, "Hello, World!", &ExistingFilePolicy::Overwrite).unwrap();
//
//             (temp_dir, file_path)
//         }
//
//         #[test]
//         fn read_existing_file() {
//             let (_temp_dir, file_path) = create_temp_file();
//
//             let file_content = read_file(&file_path).unwrap();
//
//             assert_eq!(file_content, "Hello, World!");
//         }
//
//         #[test]
//         fn reject_not_found_target() {
//             let temp_dir = tempdir().unwrap();
//             let missing_file = temp_dir.path().join("missing.txt");
//
//             let err = read_file(&missing_file).unwrap_err();
//
//             assert_eq!(err.0, PomErrorCode::FilesystemReadTargetNotFound);
//         }
//
//         #[test]
//         fn reject_directory_target() {
//             let temp_dir = tempdir().unwrap();
//             let dir_path = temp_dir.path().join("my_dir");
//
//             fs::create_dir(&dir_path).unwrap();
//
//             let err = read_file(&dir_path).unwrap_err();
//
//             assert_eq!(err.0, PomErrorCode::FilesystemReadNotFile);
//         }
//     }
// }
