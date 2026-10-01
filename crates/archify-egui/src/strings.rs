//! UI strings (tr / ar / en).

pub struct Strings {
    pub title: &'static str,
    pub explore: &'static str,
    pub stop: &'static str,
    pub fit: &'static str,
    pub search_hint: &'static str,
    pub architecture: &'static str,
    pub dataflow: &'static str,
    pub dark: &'static str,
    pub light: &'static str,
    pub ambient: &'static str,
    pub reduced: &'static str,
    pub boxes: &'static str,
    pub links: &'static str,
    pub start: &'static str,
    pub back: &'static str,
    pub hint: &'static str,
}

const TR: Strings = Strings {
    title: "Archify Mimari Stüdyosu",
    explore: "Sistemi keşfet",
    stop: "Durdur",
    fit: "Tümünü sığdır",
    search_hint: "ara…",
    architecture: "Mimari",
    dataflow: "Veri akışı",
    dark: "Koyu",
    light: "Açık",
    ambient: "Akış ışığı",
    reduced: "Hareketi azalt",
    boxes: "kutu",
    links: "bağ",
    start: "Başlangıç",
    back: "Geri",
    hint: "kaydır: pan · tekerlek: yakınlaştır · çift tık: sığdır · [ ] P: tur",
};

const EN: Strings = Strings {
    title: "Archify Architecture Studio",
    explore: "Explore this system",
    stop: "Stop",
    fit: "Fit all",
    search_hint: "search…",
    architecture: "Architecture",
    dataflow: "Dataflow",
    dark: "Dark",
    light: "Light",
    ambient: "Ambient flow",
    reduced: "Reduce motion",
    boxes: "boxes",
    links: "links",
    start: "Start",
    back: "Back",
    hint: "drag: pan · wheel: zoom · double click: fit · [ ] P: tour",
};

const AR: Strings = Strings {
    title: "استوديو بنية النظام",
    explore: "استكشف النظام",
    stop: "إيقاف",
    fit: "ملاءمة",
    search_hint: "بحث…",
    architecture: "البنية",
    dataflow: "تدفق البيانات",
    dark: "داكن",
    light: "فاتح",
    ambient: "تدفق الضوء",
    reduced: "تقليل الحركة",
    boxes: "عقدة",
    links: "رابط",
    start: "بداية",
    back: "رجوع",
    hint: "اسحب: تحريك · العجلة: تكبير · نقر مزدوج: ملاءمة",
};

pub fn strings(locale: &str) -> &'static Strings {
    match locale {
        "tr" => &TR,
        "ar" => &AR,
        _ => &EN,
    }
}
