#![allow(dead_code)]

use std::fs::{write, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fmt, fs};

use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use etcetera::base_strategy::BaseStrategy;
use indexmap::IndexMap;
use merge::Merge;
use regex::Regex;
use regex_split::RegexSplit;
use rust_i18n::t;
use serde::Deserialize;
use strum::{EnumIter, EnumString, IntoEnumIterator, VariantNames};
use which_crate::which;

use super::utils::editor;
use crate::command::CommandExt;
use crate::sudo::SudoKind;
use crate::utils::string_prepend_str;
use tracing::{debug, error};

// TODO: Add i18n to this. Tracking issue: https://github.com/topgrade-rs/topgrade/issues/859
pub static EXAMPLE_CONFIG: &str = include_str!("../config.example.toml");

/// Topgrade's default log level.
pub const DEFAULT_LOG_LEVEL: &str = "warn";

#[allow(unused_macros)]
macro_rules! str_value {
    ($section:ident, $value:ident) => {
        pub fn $value(&self) -> Option<&str> {
            self.config_file
                .$section
                .as_ref()
                .and_then(|section| section.$value.as_deref())
        }
    };
}

pub type Commands = IndexMap<String, String>;

#[derive(ValueEnum, EnumString, VariantNames, Debug, Clone, PartialEq, Eq, Deserialize, EnumIter, Copy)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Step {
    AM,
    AndroidStudio,
    AppMan,
    Aqua,
    Asdf,
    Atom,
    Audit,
    AutoCpufreq,
    Bin,
    Bob,
    BrewCask,
    BrewFormula,
    Bun,
    BunPackages,
    Cargo,
    Certbot,
    Chezmoi,
    Chocolatey,
    Choosenim,
    CinnamonSpices,
    ClamAvDb,
    Composer,
    Conda,
    ConfigUpdate,
    Containers,
    CustomCommands,
    DebGet,
    Deno,
    Distrobox,
    DkpPacman,
    Dotnet,
    Elan,
    Emacs,
    Firmware,
    Flatpak,
    Flutter,
    Fossil,
    Gcloud,
    Gem,
    Ghcup,
    GitRepos,
    GithubCliExtensions,
    GnomeShellExtensions,
    Go,
    Guix,
    Haxelib,
    Helix,
    Helm,
    HomeManager,
    // These names are miscapitalized on purpose, so the CLI name is
    //  `jetbrains_pycharm` instead of `jet_brains_py_charm`.
    JetbrainsAqua,
    JetbrainsClion,
    JetbrainsDatagrip,
    JetbrainsDataspell,
    JetbrainsGateway,
    JetbrainsGoland,
    JetbrainsIdea,
    JetbrainsMps,
    JetbrainsPhpstorm,
    JetbrainsPycharm,
    JetbrainsRider,
    JetbrainsRubymine,
    JetbrainsRustrover,
    JetbrainsToolbox,
    JetbrainsWebstorm,
    Jetpack,
    Julia,
    Juliaup,
    Kakoune,
    Krew,
    Lensfun,
    Lure,
    Macports,
    Mamba,
    Mas,
    Maza,
    Micro,
    MicrosoftStore,
    Miktex,
    Mise,
    Myrepos,
    Nix,
    Node,
    Opam,
    Pacdef,
    Pacstall,
    Pearl,
    Pip3,
    PipReview,
    PipReviewLocal,
    Pipupgrade,
    Pipx,
    Pipxu,
    Pixi,
    Pkg,
    Pkgin,
    PlatformioCore,
    Pnpm,
    Poetry,
    Powershell,
    Protonup,
    Pyenv,
    Raco,
    Rcm,
    Remotes,
    Restarts,
    Rtcl,
    RubyGems,
    Rustup,
    Rye,
    Scoop,
    Sdkman,
    SelfUpdate,
    Sheldon,
    Shell,
    Snap,
    Sparkle,
    Spicetify,
    Stack,
    Stew,
    System,
    Tldr,
    Tlmgr,
    Tmux,
    Toolbx,
    Uv,
    Vagrant,
    Vcpkg,
    Vim,
    VoltaPackages,
    Vscode,
    Vscodium,
    Waydroid,
    Winget,
    Wsl,
    WslUpdate,
    Xcodes,
    Yadm,
    Yarn,
    Yazi,
    Zigup,
    Zvm,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Include {
    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    paths: Option<Vec<String>>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    ignored_containers: Option<Vec<String>>,
    runtime: Option<ContainerRuntime>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Git {
    max_concurrency: Option<usize>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    repos: Option<Vec<String>>,

    pull_predefined: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Vagrant {
    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    directories: Option<Vec<String>>,

    power_on: Option<bool>,
    always_suspend: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Windows {
    accept_all_updates: Option<bool>,
    self_rename: Option<bool>,
    open_remotes_in_new_terminal: Option<bool>,
    wsl_update_pre_release: Option<bool>,
    wsl_update_use_web_download: Option<bool>,
    winget_silent_install: Option<bool>,
    winget_use_sudo: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Python {
    enable_pip_review: Option<bool>,
    enable_pip_review_local: Option<bool>,
    enable_pipupgrade: Option<bool>,
    pipupgrade_arguments: Option<String>,
    poetry_force_self_update: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct Distrobox {
    use_root: Option<bool>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    containers: Option<Vec<String>>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct Yarn {
    use_sudo: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct NPM {
    use_sudo: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct Deno {
    version: Option<String>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct Firmware {
    upgrade: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub struct Flatpak {
    use_sudo: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Brew {
    greedy_cask: Option<bool>,
    greedy_latest: Option<bool>,
    greedy_auto_updates: Option<bool>,
    autoremove: Option<bool>,
    fetch_head: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ArchPackageManager {
    Autodetect,
    Aura,
    GarudaUpdate,
    Pacman,
    Pamac,
    Paru,
    Pikaur,
    Trizen,
    Yay,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerRuntime::Docker => write!(f, "docker"),
            ContainerRuntime::Podman => write!(f, "podman"),
        }
    }
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Linux {
    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    yay_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    aura_aur_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    aura_pacman_arguments: Option<String>,
    arch_package_manager: Option<ArchPackageManager>,
    show_arch_news: Option<bool>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    garuda_update_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    trizen_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    pikaur_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    pamac_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    dnf_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    nix_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    nix_env_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    apt_arguments: Option<String>,

    enable_tlmgr: Option<bool>,
    redhat_distro_sync: Option<bool>,
    suse_dup: Option<bool>,
    rpm_ostree: Option<bool>,
    bootc: Option<bool>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    emerge_sync_flags: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    emerge_update_flags: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    home_manager_arguments: Option<Vec<String>>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Composer {
    self_update: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Vim {
    force_plug_update: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Misc {
    pre_sudo: Option<bool>,

    sudo_command: Option<SudoKind>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    disable: Option<Vec<Step>>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    ignore_failures: Option<Vec<Step>>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    remote_topgrades: Option<Vec<String>>,

    remote_topgrade_path: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    ssh_arguments: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::string_append_opt)]
    tmux_arguments: Option<String>,

    set_title: Option<bool>,

    display_time: Option<bool>,

    assume_yes: Option<bool>,

    no_retry: Option<bool>,

    run_in_tmux: Option<bool>,

    tmux_session_mode: Option<TmuxSessionMode>,

    cleanup: Option<bool>,

    notify_each_step: Option<bool>,

    skip_notify: Option<bool>,

    bashit_branch: Option<String>,

    #[merge(strategy = crate::utils::merge_strategies::vec_prepend_opt)]
    only: Option<Vec<Step>>,

    no_self_update: Option<bool>,

    log_filters: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, ValueEnum)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TmuxSessionMode {
    AttachIfNotInSession,
    AttachAlways,
}

pub struct TmuxConfig {
    pub args: Vec<String>,
    pub session_mode: TmuxSessionMode,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Lensfun {
    use_sudo: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct JuliaConfig {
    startup_file: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct Zigup {
    target_versions: Option<Vec<String>>,
    install_dir: Option<String>,
    path_link: Option<String>,
    cleanup: Option<bool>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
pub struct VscodeConfig {
    profile: Option<String>,
}

#[derive(Deserialize, Default, Debug, Merge)]
#[serde(deny_unknown_fields)]
/// Configuration file
pub struct ConfigFile {
    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    include: Option<Include>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    misc: Option<Misc>,

    #[merge(strategy = crate::utils::merge_strategies::commands_merge_opt)]
    pre_commands: Option<Commands>,

    #[merge(strategy = crate::utils::merge_strategies::commands_merge_opt)]
    post_commands: Option<Commands>,

    #[merge(strategy = crate::utils::merge_strategies::commands_merge_opt)]
    commands: Option<Commands>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    python: Option<Python>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    composer: Option<Composer>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    brew: Option<Brew>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    linux: Option<Linux>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    git: Option<Git>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    containers: Option<Containers>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    windows: Option<Windows>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    npm: Option<NPM>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    yarn: Option<Yarn>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    deno: Option<Deno>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    vim: Option<Vim>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    firmware: Option<Firmware>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    vagrant: Option<Vagrant>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    flatpak: Option<Flatpak>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    distrobox: Option<Distrobox>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    lensfun: Option<Lensfun>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    julia: Option<JuliaConfig>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    zigup: Option<Zigup>,

    #[merge(strategy = crate::utils::merge_strategies::inner_merge_opt)]
    vscode: Option<VscodeConfig>,
}

fn config_directory() -> PathBuf {
    #[cfg(unix)]
    return crate::XDG_DIRS.config_dir();

    #[cfg(windows)]
    return crate::WINDOWS_DIRS.config_dir();
}

/// The only purpose of this struct is to deserialize only the `include` field of the config file.
#[derive(Deserialize, Default, Debug)]
struct ConfigFileIncludeOnly {
    include: Option<Include>,
}

impl ConfigFile {
    /// Returns the main config file and any additional config files
    /// 0 = main config file
    /// 1 = additional config files coming from topgrade.d
    fn ensure() -> Result<(PathBuf, Vec<PathBuf>)> {
        let mut res = (PathBuf::new(), Vec::new());

        let config_directory = config_directory();

        let possible_config_paths = [
            config_directory.join("topgrade.toml"),
            config_directory.join("topgrade/topgrade.toml"),
        ];

        // Search for the main config file
        for path in &possible_config_paths {
            if path.exists() {
                debug!("Configuration at {}", path.display());
                res.0.clone_from(path);
                break;
            }
        }

        res.1 = Self::ensure_topgrade_d(&config_directory)?;

        // If no config file exists, create a default one in the config directory
        if !res.0.exists() && res.1.is_empty() {
            res.0.clone_from(&possible_config_paths[0]);
            debug!("No configuration exists");
            write(&res.0, EXAMPLE_CONFIG).map_err(|e| {
                debug!(
                    "Unable to write the example configuration file to {}: {}. Using blank config.",
                    &res.0.display(),
                    e
                );
                e
            })?;
        }

        Ok(res)
    }

    /// Searches topgrade.d for additional config files
    fn ensure_topgrade_d(config_directory: &Path) -> Result<Vec<PathBuf>> {
        let mut res = Vec::new();
        let dir_to_search = config_directory.join("topgrade.d");

        if dir_to_search.exists() {
            for entry in fs::read_dir(dir_to_search)? {
                let entry = entry?;
                // Use `Path::is_file()` here to traverse symbolic links.
                // `DirEntry::file_type()` and `FileType::is_file()` will not traverse symbolic links.
                if entry.path().is_file() {
                    debug!(
                        "Found additional (directory) configuration file at {}",
                        entry.path().display()
                    );
                    res.push(entry.path());
                }
            }
            res.sort();
        } else {
            debug!("No additional configuration directory exists, creating one");
            fs::create_dir_all(&dir_to_search)?;
        }

        Ok(res)
    }

    /// Read the configuration file.
    ///
    /// If the configuration file does not exist, the function returns the default ConfigFile.
    fn read(config_path: Option<PathBuf>) -> Result<ConfigFile> {
        let mut result = Self::default();

        let config_path = if let Some(path) = config_path {
            path
        } else {
            let (path, dir_include) = Self::ensure()?;

            /*
            The Function was called without a config_path, we need
            to read the include directory before returning the main config path
            */
            for include in dir_include {
                let include_contents = fs::read_to_string(&include).inspect_err(|_| {
                    error!("Unable to read {}", include.display());
                })?;
                let include_contents_parsed = toml::from_str(include_contents.as_str()).inspect_err(|_| {
                    error!("Failed to deserialize {}", include.display());
                })?;

                result.merge(include_contents_parsed);
            }

            path
        };

        if config_path == PathBuf::default() {
            // Here we expect topgrade.d and consequently result is not empty.
            // If empty, Self:: ensure() would have created the default config.
            return Ok(result);
        }

        let mut contents_non_split = fs::read_to_string(&config_path).inspect_err(|_| {
            error!("Unable to read {}", config_path.display());
        })?;

        Self::ensure_misc_is_present(&mut contents_non_split, &config_path);

        // To parse [include] sections in the order as they are written,
        // we split the file and parse each part as a separate file
        let regex_match_include = Regex::new(r"^\s*\[include]").expect("Failed to compile regex");
        let contents_split = regex_match_include.split_inclusive_left(contents_non_split.as_str());

        for contents in contents_split {
            let config_file_include_only: ConfigFileIncludeOnly = toml::from_str(contents).inspect_err(|_| {
                error!("Failed to deserialize an include section of {}", config_path.display());
            })?;

            if let Some(includes) = &config_file_include_only.include {
                // Parses the [include] section present in the slice
                if let Some(ref paths) = includes.paths {
                    for include in paths.iter().rev() {
                        let include_path = shellexpand::tilde::<&str>(&include.as_ref()).into_owned();
                        let include_path = PathBuf::from(include_path);
                        let include_contents = match fs::read_to_string(&include_path) {
                            Ok(c) => c,
                            Err(e) => {
                                error!("Unable to read {}: {e}", include_path.display(),);
                                continue;
                            }
                        };
                        match toml::from_str::<Self>(&include_contents) {
                            Ok(include_parsed) => result.merge(include_parsed),
                            Err(e) => {
                                error!("Failed to deserialize {}: {e}", include_path.display(),);
                                continue;
                            }
                        };
                    }
                }
            }

            match toml::from_str::<Self>(contents) {
                Ok(contents) => result.merge(contents),
                Err(e) => error!("Failed to deserialize {}: {e}", config_path.display(),),
            }
        }

        if let Some(paths) = result.git.as_mut().and_then(|git| git.repos.as_mut()) {
            for path in paths.iter_mut() {
                let expanded = shellexpand::tilde::<&str>(&path.as_ref()).into_owned();
                debug!(
                    "{}",
                    t!("Path {path} expanded to {expanded}", path = path, expanded = expanded)
                );
                *path = expanded;
            }
        }

        debug!("Loaded configuration: {:?}", result);
        Ok(result)
    }

    fn edit() -> Result<()> {
        let config_path = Self::ensure()?.0;
        let editor = editor();
        debug!("Editor: {:?}", editor);

        let command = which(&editor[0])?;
        let args: Vec<&String> = editor.iter().skip(1).collect();

        Command::new(command)
            .args(args)
            .arg(config_path)
            .status_checked()
            .context("Failed to open configuration file editor")
    }

    /// [Misc] was added later, here we check if it is present in the config file and add it if not
    fn ensure_misc_is_present(contents: &mut String, path: &PathBuf) {
        if !contents.contains("[misc]") {
            debug!("Adding [misc] section to {}", path.display());
            string_prepend_str(contents, "[misc]\n");

            File::create(path)
                .and_then(|mut f| f.write_all(contents.as_bytes()))
                .expect("Tried to auto-migrate the config file, unable to write to config file.\nPlease add \"[misc]\" section manually to the first line of the file.\nError");
        }
    }
}

// Command line arguments
// TODO: i18n of clap currently not easily possible. Waiting for https://github.com/clap-rs/clap/issues/380
// Tracking issue for i18n: https://github.com/topgrade-rs/topgrade/issues/859
#[derive(Parser, Debug)]
#[command(name = "topgrade", version)]
pub struct CommandLineArgs {
    /// Edit the configuration file
    #[arg(long = "edit-config")]
    edit_config: bool,

    /// Show config reference
    #[arg(long = "config-reference")]
    show_config_reference: bool,

    /// Run inside tmux
    #[arg(short = 't', long = "tmux")]
    run_in_tmux: bool,

    /// Cleanup temporary or old files
    #[arg(short = 'c', long = "cleanup")]
    cleanup: bool,

    /// Print what would be done
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,

    /// Do not ask to retry failed steps
    #[arg(long = "no-retry")]
    no_retry: bool,

    /// Do not perform upgrades for the given steps
    #[arg(long = "disable", value_name = "STEP", value_enum, num_args = 1..)]
    disable: Vec<Step>,

    /// Perform only the specified steps
    #[arg(long = "only", value_name = "STEP", value_enum, num_args = 1..)]
    only: Vec<Step>,

    /// Run only specific custom commands
    #[arg(long = "custom-commands", value_name = "NAME", num_args = 1..)]
    custom_commands: Vec<String>,

    /// Set environment variables
    #[arg(long = "env", value_name = "NAME=VALUE", num_args = 1..)]
    env: Vec<String>,

    /// Output debug logs. Alias for `--log-filter debug`.
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Prompt for a key before exiting
    #[arg(short = 'k', long = "keep")]
    keep_at_end: bool,

    /// Skip sending a notification at the end of a run
    #[arg(long = "skip-notify")]
    skip_notify: bool,

    /// Say yes to package manager's prompt
    #[arg(
        short = 'y',
        long = "yes",
        value_name = "STEP",
        value_enum,
        num_args = 0..,
    )]
    yes: Option<Vec<Step>>,

    /// Don't pull the predefined git repos
    #[arg(long = "disable-predefined-git-repos")]
    disable_predefined_git_repos: bool,

    /// Alternative configuration file
    #[arg(long = "config", value_name = "PATH")]
    config: Option<PathBuf>,

    /// A regular expression for restricting remote host execution
    #[arg(long = "remote-host-limit", value_name = "REGEX")]
    remote_host_limit: Option<Regex>,

    /// Show the reason for skipped steps
    #[arg(long = "show-skipped")]
    show_skipped: bool,

    /// Tracing filter directives.
    ///
    /// See: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
    #[arg(long, default_value = DEFAULT_LOG_LEVEL)]
    pub log_filter: String,

    /// Print completion script for the given shell and exit
    #[arg(long, value_enum, hide = true)]
    pub gen_completion: Option<Shell>,

    /// Print roff manpage and exit
    #[arg(long, hide = true)]
    pub gen_manpage: bool,

    /// Don't update Topgrade
    #[arg(long = "no-self-update")]
    pub no_self_update: bool,
}

impl CommandLineArgs {
    pub fn edit_config(&self) -> bool {
        self.edit_config
    }

    pub fn show_config_reference(&self) -> bool {
        self.show_config_reference
    }

    pub fn env_variables(&self) -> &Vec<String> {
        &self.env
    }

    /// In Topgrade, filter directives come from 3 places:
    ///     1. CLI option `--log-filter`
    ///     2. Config file
    ///     3. `debug` if the `--verbose` option is present
    ///
    /// Before loading the configuration file, we need our logger to work, so this
    /// function will return directives coming from part 1 and 2.
    ///
    ///
    /// When the configuration file is loaded, `Config::tracing_filter_directives()`
    /// will return all the 3 parts.
    pub fn tracing_filter_directives(&self) -> String {
        let mut ret = self.log_filter.clone();
        if self.verbose {
            ret.push(',');
            ret.push_str("debug");
        }

        ret
    }
}

/// Represents the application configuration
///
/// The struct holds the loaded configuration file, as well as the arguments parsed from the command line.
/// Its provided methods decide the appropriate options based on combining the configuration file and the
/// command line arguments.
#[derive(Debug)]
pub struct Config {
    opt: CommandLineArgs,
    config_file: ConfigFile,
    allowed_steps: Vec<Step>,
}

impl Config {
    /// Load the configuration.
    ///
    /// The function parses the command line arguments and reads the configuration file.
    pub fn load(opt: CommandLineArgs) -> Result<Self> {
        let config_directory = config_directory();
        let config_file = if config_directory.is_dir() {
            ConfigFile::read(opt.config.clone()).unwrap_or_else(|e| {
                // Inform the user about errors when loading the configuration,
                // but fallback to the default config to at least attempt to do something
                error!("failed to load configuration: {e}");
                ConfigFile::default()
            })
        } else {
            debug!("Configuration directory {} does not exist", config_directory.display());
            ConfigFile::default()
        };

        let allowed_steps = Self::allowed_steps(&opt, &config_file);

        Ok(Self {
            opt,
            config_file,
            allowed_steps,
        })
    }

    /// Launch an editor to edit the configuration
    pub fn edit() -> Result<()> {
        ConfigFile::edit()
    }

    /// The list of commands to run before performing any step.
    pub fn pre_commands(&self) -> &Option<Commands> {
        &self.config_file.pre_commands
    }

    /// The list of commands to run at the end of all steps
    pub fn post_commands(&self) -> &Option<Commands> {
        &self.config_file.post_commands
    }

    /// The list of custom steps.
    pub fn commands(&self) -> &Option<Commands> {
        &self.config_file.commands
    }

    /// The list of additional git repositories to pull.
    pub fn git_repos(&self) -> Option<&Vec<String>> {
        self.config_file.git.as_ref().and_then(|git| git.repos.as_ref())
    }

    /// The list of docker/podman containers to ignore.
    pub fn containers_ignored_tags(&self) -> Option<&Vec<String>> {
        self.config_file
            .containers
            .as_ref()
            .and_then(|containers| containers.ignored_containers.as_ref())
    }

    /// The preferred runtime for container updates (podman / docker).
    pub fn containers_runtime(&self) -> ContainerRuntime {
        self.config_file
            .containers
            .as_ref()
            .and_then(|containers| containers.runtime)
            .unwrap_or(ContainerRuntime::Docker) // defaults to a popular choice
    }

    /// Tell whether the specified step should run.
    ///
    /// If the step appears either in the `--disable` command line argument
    /// or the `disable` option in the configuration, the function returns false.
    pub fn should_run(&self, step: Step) -> bool {
        self.allowed_steps.contains(&step)
    }

    fn allowed_steps(opt: &CommandLineArgs, config_file: &ConfigFile) -> Vec<Step> {
        let mut enabled_steps: Vec<Step> = Vec::new();
        enabled_steps.extend(&opt.only);

        if let Some(misc) = config_file.misc.as_ref() {
            if let Some(only) = misc.only.as_ref() {
                enabled_steps.extend(only);
            }
        }

        if enabled_steps.is_empty() {
            enabled_steps.extend(Step::iter());
        }

        let mut disabled_steps: Vec<Step> = Vec::new();
        disabled_steps.extend(&opt.disable);
        if let Some(misc) = config_file.misc.as_ref() {
            if let Some(disabled) = misc.disable.as_ref() {
                disabled_steps.extend(disabled);
            }
        }

        enabled_steps.retain(|e| !disabled_steps.contains(e) || opt.only.contains(e));
        enabled_steps
    }

    /// Tell whether we should run a self-update.
    pub fn no_self_update(&self) -> bool {
        self.opt.no_self_update
            || self
                .config_file
                .misc
                .as_ref()
                .and_then(|misc| misc.no_self_update)
                .unwrap_or(false)
    }

    /// Tell whether we should run in tmux.
    pub fn run_in_tmux(&self) -> bool {
        self.opt.run_in_tmux
            || self
                .config_file
                .misc
                .as_ref()
                .and_then(|misc| misc.run_in_tmux)
                .unwrap_or(false)
    }

    /// The preferred way to run the new tmux session.
    fn tmux_session_mode(&self) -> TmuxSessionMode {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.tmux_session_mode)
            .unwrap_or(TmuxSessionMode::AttachIfNotInSession)
    }

    /// Tell whether we should perform cleanup steps.
    pub fn cleanup(&self) -> bool {
        self.opt.cleanup
            || self
                .config_file
                .misc
                .as_ref()
                .and_then(|misc| misc.cleanup)
                .unwrap_or(false)
    }

    /// Tell whether we are dry-running.
    pub fn dry_run(&self) -> bool {
        self.opt.dry_run
    }

    /// Tell whether we should not attempt to retry anything.
    pub fn no_retry(&self) -> bool {
        self.opt.no_retry
            || self
                .config_file
                .misc
                .as_ref()
                .and_then(|misc| misc.no_retry)
                .unwrap_or(false)
    }

    /// List of remote hosts to run Topgrade in
    pub fn remote_topgrades(&self) -> Option<&Vec<String>> {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.remote_topgrades.as_ref())
    }

    /// Path to Topgrade executable used for all remote hosts
    pub fn remote_topgrade_path(&self) -> &str {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.remote_topgrade_path.as_deref())
            .unwrap_or("topgrade")
    }

    /// Extra SSH arguments
    pub fn ssh_arguments(&self) -> Option<&String> {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.ssh_arguments.as_ref())
    }

    /// Extra Git arguments
    pub fn git_arguments(&self) -> Option<&String> {
        self.config_file.git.as_ref().and_then(|git| git.arguments.as_ref())
    }

    pub fn tmux_config(&self) -> Result<TmuxConfig> {
        let args = self.tmux_arguments()?;
        Ok(TmuxConfig {
            args,
            session_mode: self.tmux_session_mode(),
        })
    }

    /// Extra Tmux arguments
    fn tmux_arguments(&self) -> Result<Vec<String>> {
        let args = &self
            .config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.tmux_arguments.as_ref())
            .map(String::to_owned)
            .unwrap_or_default();
        shell_words::split(args)
            // The only time the parse failed is in case of a missing close quote.
            // The error message looks like this:
            //     Error: Failed to parse `tmux_arguments`: `'foo`
            //
            //     Caused by:
            //         missing closing quote
            .with_context(|| format!("Failed to parse `tmux_arguments`: `{args}`"))
    }

    /// Prompt for a key before exiting
    pub fn keep_at_end(&self) -> bool {
        self.opt.keep_at_end || env::var("TOPGRADE_KEEP_END").is_ok()
    }

    /// Skip sending a notification at the end of a run
    pub fn skip_notify(&self) -> bool {
        if let Some(yes) = self.config_file.misc.as_ref().and_then(|misc| misc.skip_notify) {
            return yes;
        }

        self.opt.skip_notify
    }

    /// Whether to set the terminal title
    pub fn set_title(&self) -> bool {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.set_title)
            .unwrap_or(true)
    }

    /// Whether to say yes to package managers
    pub fn yes(&self, step: Step) -> bool {
        if let Some(yes) = self.config_file.misc.as_ref().and_then(|misc| misc.assume_yes) {
            return yes;
        }

        if let Some(yes_list) = &self.opt.yes {
            if yes_list.is_empty() {
                return true;
            }

            return yes_list.contains(&step);
        }

        false
    }

    /// Bash-it branch
    pub fn bashit_branch(&self) -> &str {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.bashit_branch.as_deref())
            .unwrap_or("stable")
    }

    /// Whether to accept all Windows updates
    pub fn accept_all_windows_updates(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|windows| windows.accept_all_updates)
            .unwrap_or(true)
    }

    /// Whether to self rename the Topgrade executable during the run
    pub fn self_rename(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|w| w.self_rename)
            .unwrap_or(false)
    }

    // Should wsl --update should use the --pre-release flag
    pub fn wsl_update_pre_release(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|w| w.wsl_update_pre_release)
            .unwrap_or(false)
    }

    // Should wsl --update use the --web-download flag
    pub fn wsl_update_use_web_download(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|w| w.wsl_update_use_web_download)
            .unwrap_or(false)
    }

    /// Should use sudo for Winget
    pub fn winget_use_sudo(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|w| w.winget_use_sudo)
            .unwrap_or(false)
    }

    /// Whether Brew cask should be greedy
    pub fn brew_cask_greedy(&self) -> bool {
        self.config_file
            .brew
            .as_ref()
            .and_then(|c| c.greedy_cask)
            .unwrap_or(false)
    }

    /// Whether Brew cask should be greedy_latest
    pub fn brew_greedy_latest(&self) -> bool {
        self.config_file
            .brew
            .as_ref()
            .and_then(|c| c.greedy_latest)
            .unwrap_or(false)
    }

    /// Whether Brew cask should be auto_updates
    pub fn brew_greedy_auto_updates(&self) -> bool {
        self.config_file
            .brew
            .as_ref()
            .and_then(|c| c.greedy_auto_updates)
            .unwrap_or(false)
    }

    /// Whether Brew should autoremove
    pub fn brew_autoremove(&self) -> bool {
        self.config_file
            .brew
            .as_ref()
            .and_then(|c| c.autoremove)
            .unwrap_or(false)
    }

    /// Whether Brew should upgrade formulae built from the HEAD branch
    pub fn brew_fetch_head(&self) -> bool {
        self.config_file
            .brew
            .as_ref()
            .and_then(|c| c.fetch_head)
            .unwrap_or(false)
    }

    /// Whether Composer should update itself
    pub fn composer_self_update(&self) -> bool {
        self.config_file
            .composer
            .as_ref()
            .and_then(|c| c.self_update)
            .unwrap_or(false)
    }

    /// Whether to force plug update in Vim
    pub fn force_vim_plug_update(&self) -> bool {
        self.config_file
            .vim
            .as_ref()
            .and_then(|c| c.force_plug_update)
            .unwrap_or_default()
    }

    /// Whether to send a desktop notification at the beginning of every step
    pub fn notify_each_step(&self) -> bool {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.notify_each_step)
            .unwrap_or(false)
    }

    /// Extra garuda-update arguments
    pub fn garuda_update_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.garuda_update_arguments.as_deref())
            .unwrap_or("")
    }

    /// Extra trizen arguments
    pub fn trizen_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.trizen_arguments.as_deref())
            .unwrap_or("")
    }

    /// Extra Pikaur arguments
    #[allow(dead_code)]
    pub fn pikaur_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.pikaur_arguments.as_deref())
            .unwrap_or("")
    }

    /// Extra Pamac arguments
    pub fn pamac_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.pamac_arguments.as_deref())
            .unwrap_or("")
    }

    /// Show news on Arch Linux
    pub fn show_arch_news(&self) -> bool {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.show_arch_news)
            .unwrap_or(true)
    }

    /// Get the package manager of an Arch Linux system
    pub fn arch_package_manager(&self) -> ArchPackageManager {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.arch_package_manager)
            .unwrap_or(ArchPackageManager::Autodetect)
    }

    /// Extra yay arguments
    pub fn yay_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.yay_arguments.as_deref())
            .unwrap_or("")
    }

    /// Extra aura arguments for AUR and pacman
    pub fn aura_aur_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.aura_aur_arguments.as_deref())
            .unwrap_or("")
    }
    pub fn aura_pacman_arguments(&self) -> &str {
        self.config_file
            .linux
            .as_ref()
            .and_then(|s| s.aura_pacman_arguments.as_deref())
            .unwrap_or("")
    }

    /// Extra apt arguments
    pub fn apt_arguments(&self) -> Option<&str> {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.apt_arguments.as_deref())
    }

    /// Extra dnf arguments
    pub fn dnf_arguments(&self) -> Option<&str> {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.dnf_arguments.as_deref())
    }

    /// Extra nix arguments
    pub fn nix_arguments(&self) -> Option<&str> {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.nix_arguments.as_deref())
    }

    /// Extra nix-env arguments
    pub fn nix_env_arguments(&self) -> Option<&str> {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.nix_env_arguments.as_deref())
    }

    /// Extra Home Manager arguments
    pub fn home_manager(&self) -> Option<&Vec<String>> {
        self.config_file
            .linux
            .as_ref()
            .and_then(|misc| misc.home_manager_arguments.as_ref())
    }

    /// Distrobox use root
    pub fn distrobox_root(&self) -> bool {
        self.config_file
            .distrobox
            .as_ref()
            .and_then(|r| r.use_root)
            .unwrap_or(false)
    }

    /// Distrobox containers
    pub fn distrobox_containers(&self) -> Option<&Vec<String>> {
        self.config_file.distrobox.as_ref().and_then(|r| r.containers.as_ref())
    }

    /// Concurrency limit for git
    pub fn git_concurrency_limit(&self) -> Option<usize> {
        self.config_file.git.as_ref().and_then(|git| git.max_concurrency)
    }

    /// Determine whether we should power on vagrant boxes
    pub fn vagrant_power_on(&self) -> Option<bool> {
        self.config_file.vagrant.as_ref().and_then(|vagrant| vagrant.power_on)
    }

    /// Vagrant directories
    pub fn vagrant_directories(&self) -> Option<&Vec<String>> {
        self.config_file
            .vagrant
            .as_ref()
            .and_then(|vagrant| vagrant.directories.as_ref())
    }

    /// Always suspend vagrant boxes instead of powering off
    pub fn vagrant_always_suspend(&self) -> Option<bool> {
        self.config_file
            .vagrant
            .as_ref()
            .and_then(|vagrant| vagrant.always_suspend)
    }

    /// Enable tlmgr on Linux
    pub fn enable_tlmgr_linux(&self) -> bool {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.enable_tlmgr)
            .unwrap_or(false)
    }

    /// Use distro-sync in Red Hat based distributions
    pub fn redhat_distro_sync(&self) -> bool {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.redhat_distro_sync)
            .unwrap_or(false)
    }

    /// Use zypper dist-upgrade (same as distro-sync on RH) instead of update (default: false on SLE/Leap, ignored on Tumbleweed (dup is always ran))
    pub fn suse_dup(&self) -> bool {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.suse_dup)
            .unwrap_or(false)
    }

    /// Use rpm-ostree in *when rpm-ostree is detected* (default: true)
    pub fn rpm_ostree(&self) -> bool {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.rpm_ostree)
            .unwrap_or(false)
    }

    /// Use bootc in *when bootc is detected* (default: false)
    pub fn bootc(&self) -> bool {
        self.config_file
            .linux
            .as_ref()
            .and_then(|linux| linux.bootc)
            .unwrap_or(false)
    }

    /// Determine if we should ignore failures for this step
    pub fn ignore_failure(&self, step: Step) -> bool {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.ignore_failures.as_ref())
            .is_some_and(|v| v.contains(&step))
    }

    pub fn use_predefined_git_repos(&self) -> bool {
        !self.opt.disable_predefined_git_repos
            && self
                .config_file
                .git
                .as_ref()
                .and_then(|git| git.pull_predefined)
                .unwrap_or(true)
    }

    pub fn verbose(&self) -> bool {
        self.opt.verbose
    }

    /// After loading the config file, filter directives consist of 3 parts:
    ///
    ///     1. directives from the configuration file
    ///     2. directives from the CLI options `--log-filter`
    ///     3. `debug`, which would be enabled if the `--verbose` option is present
    ///
    /// Previous directive will be overwritten if a directive with the same target
    /// appear later.
    pub fn tracing_filter_directives(&self) -> String {
        let mut ret = String::new();
        if let Some(directives) = self.config_file.misc.as_ref().and_then(|m| m.log_filters.as_ref()) {
            ret.push_str(&directives.join(","));
        }
        ret.push(',');
        ret.push_str(&self.opt.log_filter);
        if self.verbose() {
            ret.push_str(",debug");
        }
        ret
    }

    pub fn show_skipped(&self) -> bool {
        self.opt.show_skipped
    }

    pub fn open_remotes_in_new_terminal(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|windows| windows.open_remotes_in_new_terminal)
            .unwrap_or(false)
    }

    pub fn winget_silent_install(&self) -> bool {
        self.config_file
            .windows
            .as_ref()
            .and_then(|windows| windows.winget_silent_install)
            .unwrap_or(true)
    }

    pub fn sudo_command(&self) -> Option<SudoKind> {
        self.config_file.misc.as_ref().and_then(|misc| misc.sudo_command)
    }

    /// If `true`, `sudo` should be called after `pre_commands` in order to elevate at the
    /// start of the session (and not in the middle).
    pub fn pre_sudo(&self) -> bool {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.pre_sudo)
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    pub fn npm_use_sudo(&self) -> bool {
        self.config_file
            .npm
            .as_ref()
            .and_then(|npm| npm.use_sudo)
            .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    pub fn yarn_use_sudo(&self) -> bool {
        self.config_file
            .yarn
            .as_ref()
            .and_then(|yarn| yarn.use_sudo)
            .unwrap_or(false)
    }

    pub fn deno_version(&self) -> Option<&str> {
        self.config_file.deno.as_ref().and_then(|deno| deno.version.as_deref())
    }

    #[cfg(target_os = "linux")]
    pub fn firmware_upgrade(&self) -> bool {
        self.config_file
            .firmware
            .as_ref()
            .and_then(|firmware| firmware.upgrade)
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    pub fn flatpak_use_sudo(&self) -> bool {
        self.config_file
            .flatpak
            .as_ref()
            .and_then(|flatpak| flatpak.use_sudo)
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    str_value!(linux, emerge_sync_flags);

    #[cfg(target_os = "linux")]
    str_value!(linux, emerge_update_flags);

    pub fn should_execute_remote(&self, hostname: Result<String>, remote: &str) -> bool {
        let remote_host = remote.split_once('@').map_or(remote, |(_, host)| host);

        if let Ok(hostname) = hostname {
            if remote_host == hostname {
                return false;
            }
        }

        if let Some(limit) = &self.opt.remote_host_limit.as_ref() {
            return limit.is_match(remote_host);
        }

        true
    }

    pub fn enable_pipupgrade(&self) -> bool {
        self.config_file
            .python
            .as_ref()
            .and_then(|python| python.enable_pipupgrade)
            .unwrap_or(false)
    }
    pub fn pipupgrade_arguments(&self) -> &str {
        self.config_file
            .python
            .as_ref()
            .and_then(|s| s.pipupgrade_arguments.as_deref())
            .unwrap_or("")
    }
    pub fn enable_pip_review(&self) -> bool {
        self.config_file
            .python
            .as_ref()
            .and_then(|python| python.enable_pip_review)
            .unwrap_or(false)
    }
    pub fn enable_pip_review_local(&self) -> bool {
        self.config_file
            .python
            .as_ref()
            .and_then(|python| python.enable_pip_review_local)
            .unwrap_or(false)
    }
    pub fn poetry_force_self_update(&self) -> bool {
        self.config_file
            .python
            .as_ref()
            .and_then(|python| python.poetry_force_self_update)
            .unwrap_or(false)
    }

    pub fn display_time(&self) -> bool {
        self.config_file
            .misc
            .as_ref()
            .and_then(|misc| misc.display_time)
            .unwrap_or(true)
    }

    pub fn should_run_custom_command(&self, name: &str) -> bool {
        if self.opt.custom_commands.is_empty() {
            return true;
        }

        self.opt.custom_commands.iter().any(|s| s == name)
    }

    pub fn lensfun_use_sudo(&self) -> bool {
        self.config_file
            .lensfun
            .as_ref()
            .and_then(|lensfun| lensfun.use_sudo)
            .unwrap_or(false)
    }

    pub fn julia_use_startup_file(&self) -> bool {
        self.config_file
            .julia
            .as_ref()
            .and_then(|julia| julia.startup_file)
            .unwrap_or(true)
    }

    pub fn zigup_target_versions(&self) -> Vec<String> {
        self.config_file
            .zigup
            .as_ref()
            .and_then(|zigup| zigup.target_versions.clone())
            .unwrap_or(vec!["master".to_owned()])
    }

    pub fn zigup_install_dir(&self) -> Option<&str> {
        self.config_file
            .zigup
            .as_ref()
            .and_then(|zigup| zigup.install_dir.as_deref())
    }

    pub fn zigup_path_link(&self) -> Option<&str> {
        self.config_file
            .zigup
            .as_ref()
            .and_then(|zigup| zigup.path_link.as_deref())
    }

    pub fn zigup_cleanup(&self) -> bool {
        self.config_file
            .zigup
            .as_ref()
            .and_then(|zigup| zigup.cleanup)
            .unwrap_or(false)
    }

    pub fn vscode_profile(&self) -> Option<&str> {
        let vscode_cfg = self.config_file.vscode.as_ref()?;
        let profile = vscode_cfg.profile.as_ref()?;

        if profile.is_empty() {
            None
        } else {
            Some(profile.as_str())
        }
    }
}

#[cfg(test)]
mod test {

    use crate::config::*;
    use color_eyre::eyre::eyre;

    /// Test the default configuration in `config.example.toml` is valid.
    #[test]
    fn test_default_config() {
        let str = include_str!("../config.example.toml");

        assert!(toml::from_str::<ConfigFile>(str).is_ok());
    }

    fn config() -> Config {
        Config {
            opt: CommandLineArgs::parse_from::<_, String>([]),
            config_file: ConfigFile::default(),
            allowed_steps: Vec::new(),
        }
    }

    #[test]
    fn test_should_execute_remote_different_hostname() {
        assert!(config().should_execute_remote(Ok("hostname".to_string()), "remote_hostname"));
    }

    #[test]
    fn test_should_execute_remote_different_hostname_with_user() {
        assert!(config().should_execute_remote(Ok("hostname".to_string()), "user@remote_hostname"));
    }

    #[test]
    fn test_should_execute_remote_unknown_hostname() {
        assert!(config().should_execute_remote(Err(eyre!("failed to get hostname")), "remote_hostname"));
    }

    #[test]
    fn test_should_not_execute_remote_same_hostname() {
        assert!(!config().should_execute_remote(Ok("hostname".to_string()), "hostname"));
    }

    #[test]
    fn test_should_not_execute_remote_same_hostname_with_user() {
        assert!(!config().should_execute_remote(Ok("hostname".to_string()), "user@hostname"));
    }

    #[test]
    fn test_should_execute_remote_matching_limit() {
        let mut config = config();
        config.opt = CommandLineArgs::parse_from(["topgrade", "--remote-host-limit", "remote_hostname"]);
        assert!(config.should_execute_remote(Ok("hostname".to_string()), "user@remote_hostname"));
    }

    #[test]
    fn test_should_not_execute_remote_not_matching_limit() {
        let mut config = config();
        config.opt = CommandLineArgs::parse_from(["topgrade", "--remote-host-limit", "other_hostname"]);
        assert!(!config.should_execute_remote(Ok("hostname".to_string()), "user@remote_hostname"));
    }
}
