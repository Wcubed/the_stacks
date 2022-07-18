use bevy::asset::AssetServerSettings;
use bevy::prelude::*;
use fluent::{bundle::FluentBundle, FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use unic_langid::{langid, LanguageIdentifier};

const LOCALIZATION_ASSETS_PATH: &str = "localization";
const FLUENT_FILE_EXTENSION: &str = ".ftl";

const DEFAULT_LANGUAGE: LanguageIdentifier = langid!("en-US");

/// Definition of the supported languages.
/// Includes the language id, and the name of the language,
/// in that language (used for language selection UI).
///
/// Expects there to be a `lang-id.ftl` file in the [LOCALIZATION_ASSETS_PATH] folder for each entry.
/// If the file isn't there, then that language can't be selected.
const LANGUAGES: &[(&str, &str)] = &[("en-US", "English"), ("nl-NL", "Nederlands")];

pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(localization_loader_system);
    }
}

pub struct Localizer {
    /// All supported languages, along with their name in that language.
    languages: HashMap<
        LanguageIdentifier,
        (
            FluentBundle<Arc<FluentResource>, IntlLangMemoizer>,
            &'static str,
        ),
    >,
    /// The language that is in-use.
    current_language: LanguageIdentifier,
}

impl Localizer {
    pub fn current_language(&self) -> LanguageIdentifier {
        self.current_language.clone()
    }

    pub fn select_language(&mut self, new_language: LanguageIdentifier) {
        if self.languages.contains_key(&new_language) {
            self.current_language = new_language;
        }
    }

    pub fn language_options(&self) -> HashMap<LanguageIdentifier, &'static str> {
        self.languages
            .iter()
            .map(|(key, (_, name))| (key.clone(), <&str>::clone(name)))
            .collect()
    }

    pub fn localize(&self, id: &str) -> String {
        self.localize_with_args(id, &[])
    }

    pub fn localize_with_args(&self, id: &str, args: &[(&str, &str)]) -> String {
        // TODO (Wybe 2022-06-19): Fall back on en-US if there is missing translations.
        // TODO (Wybe 2022-06-19): Add a stage to the cargo make file that checks if all the
        //      translation files have the values that the en-US.ftl file has. To make sure
        //      there are no missing translations in other languages. (ignore the ones prefixed
        //      with '-' of course.
        //      and maybe also check that it uses the same arguments (e.g. `{$seconds}`) as the
        //      english one uses, to prevent it mising information. The variables have to be
        //      directly embedded into the translation entry (and not in referenced entries), so it should be checkable.
        let mut fluent_args = FluentArgs::new();
        for (key, value) in args {
            fluent_args.set(<&str>::clone(key), <&str>::clone(value));
        }

        let current_bundle = &self.languages[&self.current_language].0;

        if let Some(msg) = current_bundle.get_message(id) {
            if let Some(pattern) = msg.value() {
                let mut errors = vec![];
                let result =
                    current_bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);

                if !errors.is_empty() {
                    let errors_string = errors
                        .iter()
                        .enumerate()
                        // TODO (Wybe 2022-06-06): Clean up this error reporting?
                        .map(|(i, err)| format!("\n{}: {:?}", i, err))
                        .collect::<String>();

                    warn!(
                        "Errors while localizing `{}` for language `{}`, with arguments {:x?}:{}",
                        id, self.current_language, args, errors_string
                    );
                }
                result.to_string()
            } else {
                warn!(
                    "Could not localize `{}` for language `{}`",
                    id, self.current_language
                );
                id.to_string()
            }
        } else {
            warn!(
                "Could not localize `{}` for language `{}`",
                id, self.current_language
            );
            id.to_string()
        }
    }
}

/// TODO (Wybe 2022-06-06): Allow switching the language while playing the game (and auto-update things like the card titles).

/// TODO (Wybe 2022-06-06): Make localization files assets, so they can be loaded by the AssetLoader,
///       and included in the overall loading progress.
pub fn localization_loader_system(
    mut commands: Commands,
    asset_server_settings: Res<AssetServerSettings>,
) {
    let folder = asset_server_settings.asset_folder.to_owned() + "/" + LOCALIZATION_ASSETS_PATH;

    let mut languages = HashMap::new();
    for (lang_id_string, name) in LANGUAGES {
        match load_language_file(&folder, lang_id_string) {
            Ok((id, bundle)) => {
                languages.insert(id, (bundle, name.clone()));
            }
            Err(e) => {
                warn!(
                    "Language {} ({}) will not be available: {}",
                    name, lang_id_string, e
                );
            }
        }
    }
    if !languages.contains_key(&DEFAULT_LANGUAGE) {
        panic!("Default language {} is not available.", DEFAULT_LANGUAGE);
    }

    commands.insert_resource(Localizer {
        languages,
        current_language: DEFAULT_LANGUAGE,
    });
}

fn load_language_file(
    localization_folder: &str,
    lang_id_string: &str,
) -> Result<
    (
        LanguageIdentifier,
        FluentBundle<Arc<FluentResource>, IntlLangMemoizer>,
    ),
    String,
> {
    let language_file_path =
        localization_folder.to_owned() + "/" + lang_id_string + FLUENT_FILE_EXTENSION;

    let fluent_content = fs::read_to_string(language_file_path).map_err(|e| e.to_string())?;

    let resource = FluentResource::try_new(fluent_content).map_err(|(_, errors)| {
        errors
            .iter()
            .enumerate()
            // TODO (Wybe 2022-06-06): Clean up this error reporting?
            .map(|(i, err)| format!("\n{}: {:?}", i, err))
            .collect::<String>()
    })?;

    let id = lang_id_string
        .parse::<LanguageIdentifier>()
        .expect("Parsing language identifier failed");

    let mut bundle = FluentBundle::new_concurrent(vec![id.clone()]);
    bundle.add_resource(Arc::new(resource)).map_err(|errors| {
        errors
            .iter()
            .enumerate()
            .map(|(i, err)| format!("\n{}: {:?}", i, err))
            .collect::<String>()
    })?;

    Ok((id, bundle))
}
