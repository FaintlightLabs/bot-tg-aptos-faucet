use anyhow::{Context, Result};
use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource, FluentValue};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use unic_langid::LanguageIdentifier;

pub struct I18n {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
    fallback: LanguageIdentifier,
}

impl I18n {
    pub fn new<P: AsRef<Path>>(locales_dir: P) -> Result<Self> {
        let mut bundles = HashMap::new();
        let fallback: LanguageIdentifier = "en".parse().context("invalid fallback locale")?;

        for entry in fs::read_dir(&locales_dir)
            .with_context(|| format!("cannot read locales dir {:?}", locales_dir.as_ref()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ftl") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("invalid locale filename")?;
            let langid: LanguageIdentifier = stem
                .parse()
                .with_context(|| format!("invalid locale {stem}"))?;

            let source = fs::read_to_string(&path)
                .with_context(|| format!("cannot read locale file {:?}", path))?;
            let resource = FluentResource::try_new(source)
                .map_err(|(_, errs)| anyhow::anyhow!("FTL parse errors: {:?}", errs))?;

            let mut bundle = FluentBundle::new_concurrent(vec![langid.clone()]);
            bundle
                .add_resource(resource)
                .map_err(|errs| anyhow::anyhow!("FTL add_resource errors: {:?}", errs))?;
            bundles.insert(langid, bundle);
        }

        anyhow::ensure!(
            bundles.contains_key(&fallback),
            "fallback locale 'en' not found in {:?}",
            locales_dir.as_ref()
        );

        Ok(Self { bundles, fallback })
    }

    pub fn locales(&self) -> Vec<String> {
        self.bundles.keys().map(|l| l.to_string()).collect()
    }

    pub fn translate(
        &self,
        locale: &str,
        key: &str,
        args: Option<HashMap<String, String>>,
    ) -> String {
        let langid: LanguageIdentifier = match locale.parse() {
            Ok(l) if self.bundles.contains_key(&l) => l,
            _ => self.fallback.clone(),
        };
        let bundle = self
            .bundles
            .get(&langid)
            .unwrap_or_else(|| self.bundles.get(&self.fallback).unwrap());

        let mut fluent_args = FluentArgs::new();
        if let Some(args) = args {
            for (k, v) in args {
                fluent_args.set(k, FluentValue::from(v));
            }
        }

        self.format_pattern(bundle, key, Some(&fluent_args))
            .unwrap_or_else(|| key.to_string())
    }

    fn format_pattern(
        &self,
        bundle: &FluentBundle<FluentResource>,
        key: &str,
        args: Option<&FluentArgs>,
    ) -> Option<String> {
        // Fluent attributes are accessed as "message.attr".
        if let Some((message_id, attr_name)) = key.rsplit_once('.')
            && let Some(message) = bundle.get_message(message_id)
            && let Some(attr) = message.get_attribute(attr_name)
        {
            let mut errors = Vec::new();
            let value = bundle.format_pattern(attr.value(), args, &mut errors);
            return Some(value.into_owned());
        }

        let message = bundle.get_message(key)?;
        let pattern = message.value()?;
        let mut errors = Vec::new();
        let value = bundle.format_pattern(pattern, args, &mut errors);
        Some(value.into_owned())
    }
}

#[macro_export]
macro_rules! t_args {
    ( $($key:expr => $value:expr),* $(,)? ) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key.to_string(), $value.to_string());
            )*
            Some(map)
        }
    };
}
