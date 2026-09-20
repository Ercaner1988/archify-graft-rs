//! Locale strings (tr/ar/en) and word-based role inference for module-level diagrams.

use archify_ir::SemanticRole;
use graft_model::EdgeRelation;

fn pick(locale: &str, tr: &'static str, ar: &'static str, en: &'static str) -> &'static str {
    match locale {
        "tr" => tr,
        "ar" => ar,
        _ => en,
    }
}

pub fn relation_label(rel: &EdgeRelation, locale: &str) -> &'static str {
    match rel {
        EdgeRelation::Calls => pick(locale, "çağırır", "يستدعي", "calls"),
        EdgeRelation::Imports => pick(locale, "içe aktarır", "يستورد", "imports"),
        EdgeRelation::Extends => pick(locale, "genişletir", "يمتد من", "extends"),
        EdgeRelation::Implements => pick(locale, "uygular", "ينفّذ", "implements"),
        EdgeRelation::References => pick(locale, "başvurur", "يشير إلى", "references"),
        EdgeRelation::Contains => pick(locale, "içerir", "يحتوي", "contains"),
    }
}

pub fn counts_label(files: usize, symbols: usize, locale: &str) -> String {
    match locale {
        "tr" => format!("{files} dosya · {symbols} sembol"),
        "ar" => format!("{files} ملف · {symbols} رمز"),
        _ => format!("{files} files · {symbols} symbols"),
    }
}

/// `shown` of `total` modules are drawn (the rest is cut to keep the picture readable).
pub fn modules_label(shown: usize, total: usize, locale: &str) -> String {
    match locale {
        "tr" => format!("{shown}/{total} modül"),
        "ar" => format!("{shown}/{total} وحدة"),
        _ => format!("{shown}/{total} modules"),
    }
}

pub fn subtitle(shown: usize, total: usize, links: usize, locale: &str) -> String {
    match locale {
        "tr" => format!("Koddan otomatik üretildi · {shown}/{total} modül · {links} bağ"),
        "ar" => format!("مُولَّد تلقائيًا من الشيفرة · {shown}/{total} وحدة · {links} ارتباط"),
        _ => format!("Generated from code · {shown}/{total} modules · {links} links"),
    }
}

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Role from whole words only: `ui` matches `ui`/`gui-kit` but not `guide` or `build`.
fn role_from_words(s: &str) -> Option<SemanticRole> {
    let w = words(s);
    let has = |list: &[&str]| w.iter().any(|x| list.contains(&x.as_str()));
    if has(&[
        "auth",
        "token",
        "crypto",
        "security",
        "secret",
        "guvenlik",
        "güvenlik",
        "kimlik",
        "anahtar",
    ]) {
        Some(SemanticRole::Security)
    } else if has(&[
        "db",
        "sql",
        "sqlite",
        "database",
        "storage",
        "store",
        "cache",
        "index",
        "indeks",
        "veri",
        "veritabani",
        "depolama",
        "onbellek",
    ]) {
        Some(SemanticRole::Database)
    } else if has(&[
        "bus", "queue", "event", "events", "stream", "pipe", "spool", "watcher", "olay", "kuyruk",
        "boru", "izleyici",
    ]) {
        Some(SemanticRole::Messagebus)
    } else if has(&[
        "ui",
        "gui",
        "tui",
        "egui",
        "view",
        "views",
        "web",
        "frontend",
        "client",
        "page",
        "pages",
        "components",
        "theme",
        "cli",
        "cmd",
        "bin",
        "pencere",
        "arayuz",
        "arayüz",
        "ekran",
        "panel",
        "tema",
    ]) {
        Some(SemanticRole::Frontend)
    } else if has(&["cloud", "aws", "gcp", "azure", "mesh", "uzak"]) {
        Some(SemanticRole::Cloud)
    } else if has(&["external", "vendor", "sdk", "ext", "thirdparty"]) {
        Some(SemanticRole::External)
    } else {
        None
    }
}

/// Module name wins, then the region name; ordinary code is `Backend`, not grey `External`.
pub fn role_for(module: &str, region: &str) -> SemanticRole {
    role_from_words(module)
        .or_else(|| role_from_words(region))
        .unwrap_or(SemanticRole::Backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_match_whole_words_not_substrings() {
        assert_eq!(role_for("guide", "core"), SemanticRole::Backend);
        assert_eq!(role_for("build", "core"), SemanticRole::Backend);
        assert_eq!(role_for("report", "core"), SemanticRole::Backend);
        assert_eq!(role_for("gui-kit", "core"), SemanticRole::Frontend);
        assert_eq!(role_for("user_db", "core"), SemanticRole::Database);
    }

    #[test]
    fn module_role_beats_region_role_and_region_is_the_fallback() {
        assert_eq!(role_for("store", "pencere-egui"), SemanticRole::Database);
        assert_eq!(role_for("app", "pencere-egui"), SemanticRole::Frontend);
        assert_eq!(role_for("kopru", "cekirdek"), SemanticRole::Backend);
    }

    #[test]
    fn labels_follow_the_locale_and_default_to_english() {
        assert_eq!(relation_label(&EdgeRelation::Calls, "tr"), "çağırır");
        assert_eq!(relation_label(&EdgeRelation::Calls, "ar"), "يستدعي");
        assert_eq!(relation_label(&EdgeRelation::Calls, "xx"), "calls");
        assert_eq!(modules_label(3, 9, "en"), "3/9 modules");
    }
}
