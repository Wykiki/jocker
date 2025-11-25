use std::{fmt::Display, str::FromStr};

use clap::{builder::PossibleValue, ValueEnum};
use clap_complete::{shells, Generator};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Shell {
    /// Bourne Again `SHell` (bash)
    Bash,
    /// Elvish shell
    Elvish,
    /// Friendly Interactive `SHell` (fish)
    Fish,
    // Nushell (nu)
    Nushell,
    /// `PowerShell`
    #[expect(clippy::enum_variant_names)]
    PowerShell,
    /// Z `SHell` (zsh)
    Zsh,
}

impl Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl FromStr for Shell {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("invalid variant: {s}"))
    }
}

impl Generator for Shell {
    fn file_name(&self, name: &str) -> String {
        match self {
            Shell::Bash => shells::Bash.file_name(name),
            Shell::Elvish => shells::Elvish.file_name(name),
            Shell::Fish => shells::Fish.file_name(name),
            Shell::Nushell => clap_complete_nushell::Nushell.file_name(name),
            Shell::PowerShell => shells::PowerShell.file_name(name),
            Shell::Zsh => shells::Zsh.file_name(name),
        }
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        self.try_generate(cmd, buf)
            .expect("failed to write completion file");
    }

    fn try_generate(
        &self,
        cmd: &clap::Command,
        buf: &mut dyn std::io::Write,
    ) -> Result<(), std::io::Error> {
        match self {
            Shell::Bash => shells::Bash.try_generate(cmd, buf),
            Shell::Elvish => shells::Elvish.try_generate(cmd, buf),
            Shell::Fish => shells::Fish.try_generate(cmd, buf),
            Shell::Nushell => clap_complete_nushell::Nushell.try_generate(cmd, buf),
            Shell::PowerShell => shells::PowerShell.try_generate(cmd, buf),
            Shell::Zsh => shells::Zsh.try_generate(cmd, buf),
        }
    }
}

impl ValueEnum for Shell {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Shell::Bash,
            Shell::Elvish,
            Shell::Fish,
            Shell::Nushell,
            Shell::PowerShell,
            Shell::Zsh,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Shell::Bash => PossibleValue::new("bash"),
            Shell::Elvish => PossibleValue::new("elvish"),
            Shell::Fish => PossibleValue::new("fish"),
            Shell::Nushell => PossibleValue::new("nushell"),
            Shell::PowerShell => PossibleValue::new("powershell"),
            Shell::Zsh => PossibleValue::new("zsh"),
        })
    }
}
