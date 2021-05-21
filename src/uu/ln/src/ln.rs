//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Joseph Crail <jbcrail@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) srcpath targetpath EEXIST

#[macro_use]
extern crate uucore;

extern crate clap;

use std::fs;
use std::io::{stdin, Result};
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};

use clap::{App, Arg};

static NAME: &str = "ln";
static LONG_HELP: &str = "
In the 1st form, create a link to TARGET with the name LINK_NAME. 
In the 2nd form, create a link to TARGET in the current directory.
In the 3rd and 4th forms, create links to each TARGET in DIRECTORY. 
Create hard links by default, symbolic links with --symbolic. \
By default, each destination (name of new link) should not already exist. \
When creating hard links, each TARGET must exist.  Symbolic links \
can hold arbitrary text; if later resolved, a relative link is \
interpreted in relation to its parent directory.
";
static ABOUT: &str = "make links between files";
static OPT_PATHS: &str = "paths";
static VERSION: &str = env!("CARGO_PKG_VERSION");

const OPT_BACKUP_NO_ARGS: &str = "b";
const OPT_BACKUP: &str = "backup";
const OPT_FORCE: &str = "force";
const OPT_INTERACTIVE: &str = "interactive";
const OPT_SYMBOLIC: &str = "symbolic";
const OPT_SUFFIX : &str = "suffix";
const OPT_TARGET_DIRECTORY: &str = "target-directory";
const OPT_NO_TARGET_DIRECTORY: &str = "no-target-directory";
const OPT_VERBOSE: &str = "verbose";

//TODO not implemented
//TODO const OPT_DIRECTORY: &str = "directory";
//TODO const OPT_LOGICAL: &str = "logical";
//TODO const OPT_NO_DEREFERENCE: &str = "no-dereference";
//TODO const OPT_PHYSICAL: &str = "physical";
//TODO const OPT_RELATIVE: &str = "relative";

pub struct Settings {
    overwrite: OverwriteMode,
    backup: BackupMode,
    suffix: String,
    symbolic: bool,
    target_dir: Option<String>,
    no_target_dir: bool,
    verbose: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OverwriteMode {
    NoClobber,
    Interactive,
    Force,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}


fn get_usage() -> String {
    format!(
        "[OPTION]... [-T] TARGET LINK_NAME   (1st form)
 {0} [OPTION]... TARGET                  (2nd form)
 {0} [OPTION]... TARGET... DIRECTORY     (3rd form)
 {0} [OPTION]... -t DIRECTORY TARGET...  (4th form)",
        executable!()
    )
}

pub fn uumain(args: Vec<String>) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(LONG_HELP)
        .arg(Arg::with_name(OPT_BACKUP_NO_ARGS)
            .short(OPT_BACKUP_NO_ARGS)
            .takes_value(false)
            .help("like --backup but does not accept an argument"))
        .arg(Arg::with_name(OPT_BACKUP)
            .long(OPT_BACKUP)
            .takes_value(true)
            .possible_values(&["simple","never", "numbered","t", "existing","nil", "none"])
            .help("make a backup of each file that would otherwise be \
                   overwritten or removed"))
        //TODO:
        // .arg(Arg::with_name(OPT_DIRECTORY)
        //     .short("d")
        //     .long(OPT_DIRECTORY)
        //     .help("allow users with appropriate privileges to attempt \
        //            to make hard links to directories");
        .arg(Arg::with_name(OPT_FORCE)
            .short("f")
            .long(OPT_FORCE)
            .help("remove existing destination files"))
        .arg(Arg::with_name(OPT_INTERACTIVE)
            .short("i")
            .long(OPT_INTERACTIVE)
            .help("prompt whether to remove existing destination files"))
        //TODO 
        //.arg(Arg::with_name(OPT_LOGICAL) 
        //  .short("L")
        //  .long(OPT_LOGICAL)
        //  .help("dereference TARGETs that are symbolic links"))
        //
        //TODO 
        //.arg(Arg::with_name(OPT_NO_DEREFERENCE)
        //  .short("n")
        //  .long(OPT_NO_DEREFERENCE)
        //  .help("treat LINK_NAME as a normal file if it is a symbolic link to a directory"))
        //
        //TODO 
        //.arg(Arg::with_name(OPT_PHYSICAL)
        //  .short("P") 
        //  .long(OPT_PHYSICAL) 
        //  .help("make hard links directly to symbolic links"))
        //
        //TODO 
        //  .arg(Arg::with_name(OPT_RELATIVE)
        //  .short("r")
        //  .long(OPT_RELATIVE)
        //  .help("create symbolic links relative to link location"))
        .arg(Arg::with_name(OPT_SYMBOLIC)
            .short("s")
            .long(OPT_SYMBOLIC)
            .help("create symbolic links relative to link location"))
        .arg(Arg::with_name(OPT_SUFFIX)
            .short("S")
            .long(OPT_SUFFIX)
            .takes_value(true)
            .default_value("~")
            .help("override the usual backup suffix"))
        .arg(Arg::with_name(OPT_TARGET_DIRECTORY)
            .short("t")
            .long(OPT_TARGET_DIRECTORY)
            .takes_value(true)
            .conflicts_with(OPT_NO_TARGET_DIRECTORY)
            .help("specify the DIRECTORY in which to create the links"))
        .arg(Arg::with_name(OPT_NO_TARGET_DIRECTORY)
            .short("T")
            .long(OPT_NO_TARGET_DIRECTORY)
            .conflicts_with(OPT_TARGET_DIRECTORY)
            .help("treat LINK_NAME as a normal file always"))
        .arg(Arg::with_name(OPT_VERBOSE)
            .short("v")
            .long(OPT_VERBOSE)
            .help("print name of each linked file"))
        .arg(Arg::with_name(OPT_PATHS)
             .multiple(true))
        .get_matches_from(&args);

    let overwrite_mode = if matches.is_present(OPT_FORCE) {
        OverwriteMode::Force
    } else if matches.is_present(OPT_INTERACTIVE) {
        OverwriteMode::Interactive
    } else {
        OverwriteMode::NoClobber
    };

    let backup_mode = if matches.is_present(OPT_BACKUP) {
        match matches.value_of(OPT_BACKUP).unwrap(){
            "simple" | "never" => BackupMode::SimpleBackup,
            "numbered" | "t" => BackupMode::NumberedBackup,
            "existing" | "nil" => BackupMode::ExistingBackup,
            "none" | "off" => BackupMode::NoBackup,
            x => {
                show_error!(
                    "invalid argument '{}' for 'backup method'\n\
                     Try '{} --help' for more information.",
                    NAME,x
                );
                return 1;
            }
        }
    } else if matches.is_present(OPT_BACKUP_NO_ARGS) {
        BackupMode::ExistingBackup
    }
    else {
        BackupMode::NoBackup
    };
    
    let settings = Settings {
        overwrite: overwrite_mode,
        backup: backup_mode,
        suffix: matches.value_of(OPT_SUFFIX).unwrap().to_string(),
        symbolic: matches.is_present(OPT_SYMBOLIC),
        target_dir: matches.value_of(OPT_TARGET_DIRECTORY).map(ToString::to_string),
        no_target_dir: matches.is_present(OPT_NO_TARGET_DIRECTORY),
        verbose: matches.is_present(OPT_VERBOSE),
    };

    let paths: Vec<PathBuf> = matches
        .values_of(OPT_PATHS)
        .map(|v| v.map(PathBuf::from).collect())
        .unwrap_or_default();

    exec(&paths[..], &settings)
}

fn exec(files: &[PathBuf], settings: &Settings) -> i32 {
    if files.is_empty() {
        show_error!(
            "missing file operand\nTry '{} --help' for more information.",
            NAME
        );
        return 1;
    }

    // Handle cases where we create links in a directory first.
    if let Some(ref name) = settings.target_dir {
        // 4th form: a directory is specified by -t.
        return link_files_in_dir(files, &PathBuf::from(name), &settings);
    }
    if !settings.no_target_dir {
        if files.len() == 1 {
            // 2nd form: the target directory is the current directory.
            return link_files_in_dir(files, &PathBuf::from("."), &settings);
        }
        let last_file = &PathBuf::from(files.last().unwrap());
        if files.len() > 2 || last_file.is_dir() {
            // 3rd form: create links in the last argument.
            return link_files_in_dir(&files[0..files.len() - 1], last_file, &settings);
        }
    }

    // 1st form. Now there should be only two operands, but if -T is
    // specified we may have a wrong number of operands.
    if files.len() == 1 {
        show_error!(
            "missing destination file operand after '{}'",
            files[0].to_string_lossy()
        );
        return 1;
    }
    if files.len() > 2 {
        show_error!(
            "extra operand '{}'\nTry '{} --help' for more information.",
            files[2].display(),
            NAME
        );
        return 1;
    }
    assert!(!files.is_empty());

    match link(&files[0], &files[1], settings) {
        Ok(_) => 0,
        Err(e) => {
            show_error!("{}", e);
            1
        }
    }
}

fn link_files_in_dir(files: &[PathBuf], target_dir: &PathBuf, settings: &Settings) -> i32 {
    if !target_dir.is_dir() {
        show_error!("target '{}' is not a directory", target_dir.display());
        return 1;
    }

    let mut all_successful = true;
    for srcpath in files.iter() {
        let targetpath = match srcpath.as_os_str().to_str() {
            Some(name) => {
                match Path::new(name).file_name() {
                    Some(basename) => target_dir.join(basename),
                    // This can be None only for "." or "..". Trying
                    // to create a link with such name will fail with
                    // EEXIST, which agrees with the behavior of GNU
                    // coreutils.
                    None => target_dir.join(name),
                }
            }
            None => {
                show_error!(
                    "cannot stat '{}': No such file or directory",
                    srcpath.display()
                );
                all_successful = false;
                continue;
            }
        };

        if let Err(e) = link(srcpath, &targetpath, settings) {
            show_error!(
                "cannot link '{}' to '{}': {}",
                targetpath.display(),
                srcpath.display(),
                e
            );
            all_successful = false;
        }
    }
    if all_successful {
        0
    } else {
        1
    }
}

fn link(src: &PathBuf, dst: &PathBuf, settings: &Settings) -> Result<()> {
    let mut backup_path = None;

    if is_symlink(dst) || dst.exists() {
        match settings.overwrite {
            OverwriteMode::NoClobber => {}
            OverwriteMode::Interactive => {
                print!("{}: overwrite '{}'? ", NAME, dst.display());
                if !read_yes() {
                    return Ok(());
                }
                fs::remove_file(dst)?
            }
            OverwriteMode::Force => fs::remove_file(dst)?,
        };

        backup_path = match settings.backup {
            BackupMode::NoBackup => None,
            BackupMode::SimpleBackup => Some(simple_backup_path(dst, &settings.suffix)),
            BackupMode::NumberedBackup => Some(numbered_backup_path(dst)),
            BackupMode::ExistingBackup => Some(existing_backup_path(dst, &settings.suffix)),
        };
        if let Some(ref p) = backup_path {
            fs::rename(dst, p)?;
        }
    }

    if settings.symbolic {
        symlink(src, dst)?;
    } else {
        fs::hard_link(src, dst)?;
    }

    if settings.verbose {
        print!("'{}' -> '{}'", dst.display(), src.display());
        match backup_path {
            Some(path) => println!(" (backup: '{}')", path.display()),
            None => println!(),
        }
    }
    Ok(())
}

fn read_yes() -> bool {
    let mut s = String::new();
    match stdin().read_line(&mut s) {
        Ok(_) => match s.char_indices().next() {
            Some((_, x)) => x == 'y' || x == 'Y',
            _ => false,
        },
        _ => false,
    }
}

fn simple_backup_path(path: &PathBuf, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_str().unwrap().to_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &PathBuf) -> PathBuf {
    let mut i: u64 = 1;
    loop {
        let new_path = simple_backup_path(path, &format!(".~{}~", i));
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

fn existing_backup_path(path: &PathBuf, suffix: &str) -> PathBuf {
    let test_path = simple_backup_path(path, &".~1~".to_owned());
    if test_path.exists() {
        return numbered_backup_path(path);
    }
    simple_backup_path(path, suffix)
}

#[cfg(windows)]
pub fn symlink<P: AsRef<Path>>(src: P, dst: P) -> Result<()> {
    if src.as_ref().is_dir() {
        symlink_dir(src, dst)
    } else {
        symlink_file(src, dst)
    }
}

pub fn is_symlink<P: AsRef<Path>>(path: P) -> bool {
    match fs::symlink_metadata(path) {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false,
    }
}
