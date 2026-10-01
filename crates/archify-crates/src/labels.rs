//! Locale strings (tr/ar/en) and word-based role inference for the diagrams.

use archify_ir::SemanticRole;
use graft_i18n::fold_tr;
use graft_model::EdgeRelation;

pub fn pick(locale: &str, tr: &'static str, ar: &'static str, en: &'static str) -> &'static str {
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
        EdgeRelation::DependsOn => pick(locale, "kullanır", "يستخدم", "uses"),
        EdgeRelation::Reads => pick(locale, "okur", "يقرأ", "reads"),
        EdgeRelation::Writes => pick(locale, "yazar", "يكتب", "writes"),
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

pub fn crates_subtitle(shown: usize, total: usize, links: usize, locale: &str) -> String {
    match locale {
        "tr" => format!("Cargo bağımlılıklarından üretildi · {shown}/{total} sandık · {links} bağ"),
        "ar" => format!("مُولَّد من تبعيات Cargo · {shown}/{total} صندوق · {links} ارتباط"),
        _ => format!("Built from Cargo dependencies · {shown}/{total} crates · {links} links"),
    }
}

/// `uses`: number of `use` paths; 0 means a declared but unused-in-code dependency.
pub fn uses_label(uses: u32, locale: &str) -> String {
    match (uses, locale) {
        (0, "tr") => "bağımlı".to_string(),
        (0, "ar") => "يعتمد".to_string(),
        (0, _) => "depends".to_string(),
        (n, "tr") => format!("kullanır ×{n}"),
        (n, "ar") => format!("يستخدم ×{n}"),
        (n, _) => format!("uses ×{n}"),
    }
}

pub fn crate_sublabel(lib: bool, bin: bool, external: bool, lines: usize, locale: &str) -> String {
    if external {
        return pick(locale, "dış depo", "مستودع خارجي", "external repo").to_string();
    }
    let kind = match (lib, bin) {
        (true, true) => pick(locale, "kütüphane+ikili", "مكتبة+تنفيذي", "lib+bin"),
        (false, true) => pick(locale, "ikili", "تنفيذي", "binary"),
        _ => pick(locale, "kütüphane", "مكتبة", "library"),
    };
    match locale {
        "tr" => format!("{kind} · {lines} satır"),
        "ar" => format!("{kind} · {lines} سطر"),
        _ => format!("{kind} · {lines} lines"),
    }
}

pub fn group_sublabel(crates: usize, external: bool, locale: &str) -> String {
    match (external, locale) {
        (false, "tr") => format!("çalışma alanı · {crates} sandık"),
        (true, "tr") => format!("dış depo · {crates} sandık"),
        (false, "ar") => format!("مساحة عمل · {crates} صندوق"),
        (true, "ar") => format!("مستودع خارجي · {crates} صندوق"),
        (false, _) => format!("workspace · {crates} crates"),
        (true, _) => format!("external repo · {crates} crates"),
    }
}

fn words(s: &str) -> Vec<String> {
    fold_tr(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_word(w: &[String], list: &[&str]) -> bool {
    list.iter().any(|k| w.contains(&fold_tr(k)))
}

/// Role from whole words only: `ui` matches `ui`/`gui-kit` but not `guide` or `build`.
fn role_from_words(s: &str) -> Option<SemanticRole> {
    let w = words(s);
    let has = |list: &[&str]| has_word(&w, list);
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
        "kasa",
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
        "depo",
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
        "tuval",
        "canvas",
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

/// Crate role from its name and, more reliably, from what it depends on: a GUI toolkit
/// makes a UI crate, `clap` with no dependents makes an entry point.
pub fn role_for_crate(
    name: &str,
    deps: &[&str],
    bin_only: bool,
    has_dependents: bool,
) -> SemanticRole {
    let dep = |list: &[&str]| deps.iter().any(|d| list.contains(d));
    if let Some(r) = role_from_words(name) {
        return r;
    }
    let gui = dep(&[
        "egui", "eframe", "tauri", "iced", "slint", "ratatui", "dioxus", "leptos",
    ]);
    if gui || (!has_dependents && (bin_only || dep(&["clap"]))) {
        SemanticRole::Frontend
    } else {
        SemanticRole::Backend
    }
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
    fn turkish_words_fold_without_bare_lowercase() {
        assert_eq!(role_for("ARAYÜZ", "x"), SemanticRole::Frontend);
        assert_eq!(role_for("Arayuz", "x"), SemanticRole::Frontend);
        assert_eq!(role_for("GÜVENLİK", "x"), SemanticRole::Security);
        assert_eq!(role_for("GUVENLIK", "x"), SemanticRole::Security);
        assert_eq!(role_for("ISLEM-DEPO", "x"), SemanticRole::Database);
    }

    #[test]
    fn module_role_beats_region_role_and_region_is_the_fallback() {
        assert_eq!(role_for("store", "pencere-egui"), SemanticRole::Database);
        assert_eq!(role_for("app", "pencere-egui"), SemanticRole::Frontend);
        assert_eq!(role_for("kopru", "cekirdek"), SemanticRole::Backend);
    }

    #[test]
    fn crates_get_their_role_from_dependencies_and_entry_shape() {
        assert_eq!(
            role_for_crate("pasli-kod-gezgin", &["egui"], false, true),
            SemanticRole::Frontend
        );
        assert_eq!(
            role_for_crate("pasli", &["clap"], true, false),
            SemanticRole::Frontend
        );
        assert_eq!(
            role_for_crate("pasli-cekirdek", &["clap"], false, true),
            SemanticRole::Backend
        );
        assert_eq!(
            role_for_crate("fihrist-storage", &[], false, true),
            SemanticRole::Database
        );
    }

    #[test]
    fn labels_follow_the_locale_and_default_to_english() {
        assert_eq!(relation_label(&EdgeRelation::Calls, "tr"), "çağırır");
        assert_eq!(relation_label(&EdgeRelation::Calls, "ar"), "يستدعي");
        assert_eq!(relation_label(&EdgeRelation::Calls, "xx"), "calls");
        assert_eq!(modules_label(3, 9, "en"), "3/9 modules");
        assert_eq!(uses_label(39, "tr"), "kullanır ×39");
        assert_eq!(uses_label(0, "en"), "depends");
    }
}
