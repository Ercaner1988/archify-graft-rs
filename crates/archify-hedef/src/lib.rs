//! Amaç katmanı: bir deponun hedef beyanını (`.harness/feature-list.json`) tek bir
//! kayda çevirir.
//!
//! Neden ayrı bir crate? `docs/AMAC_KATMANI.md`'de ölçüldüğü gibi aynı adı taşıyan
//! sekiz dosyanın hiçbiri aynı şemada değil: 91 özellik satırı, üç adlandırma dili
//! (`name`/`title`/`baslik`/`goal`/`ad`), yedi durum sözcüğü. Bu crate o dağınıklığı
//! **ayıklamaz**, tek kayda çevirir — Graft'ın zemin katmanı da dağınıklığı bilerek
//! içeriyor (`docs/VIZYON.md`).
//!
//! Temel kural `kapi`nın kendi ilkesidir: **ölçülmedi asla yeşil sayılmaz**. Tanınmayan
//! durum sözcüğü ya da okunamayan satır `Bilinmiyor` olur ve raporda ayrı sayılır;
//! `Tamam`a yuvarlanmaz.

use std::path::{Path, PathBuf};

/// Bir hedefin durumu. Sözlük deponun kendi sözcüğünden çevrilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Durum {
    /// Bitmiş, kanıtı beyan edilmiş ya da edilmemiş.
    Tamam,
    /// Bir kısmı bitmiş.
    Kismi,
    /// Üzerinde çalışılıyor.
    Devam,
    /// Sırada, başlanmamış.
    Bekliyor,
    /// Bilerek ertelenmiş; gerekçesi `Hedef::not` alanında.
    Ertelendi,
    /// Sözcük tanınmadı ya da hiç yazılmamış: **yeşil sayılmaz**.
    Bilinmiyor,
}

impl Durum {
    /// Deponun yazdığı sözcüğü çevirir. Tanınmayan sözcük `Bilinmiyor` döner.
    ///
    /// Katlama deponun kendi `fold_tr`'siyle yapılır: `I`, `İ`, `ı` tek biçime
    /// iner, yoksa `KISMİ` gibi bir yazım sessizce tanınmaz kalırdı.
    pub fn cevir(sozcuk: &str) -> Self {
        match graft_i18n::fold_tr(sozcuk.trim()).as_str() {
            "passing" | "done" | "tamam" | "bitti" | "complete" | "completed" => Self::Tamam,
            "partial" | "kismi" => Self::Kismi,
            "in_progress" | "in-progress" | "devam" | "doing" | "active" => Self::Devam,
            "pending" | "todo" | "bekliyor" | "queued" => Self::Bekliyor,
            "deferred" | "ertelendi" | "wontdo" | "wont_do" => Self::Ertelendi,
            _ => Self::Bilinmiyor,
        }
    }

    /// Tek harflik işaret (rapor için).
    pub fn isaret(self) -> &'static str {
        match self {
            Self::Tamam => "+",
            Self::Kismi => "~",
            Self::Devam => ">",
            Self::Bekliyor => ".",
            Self::Ertelendi => "-",
            Self::Bilinmiyor => "?",
        }
    }

    /// Türkçe etiket (rapor için).
    pub fn etiket(self) -> &'static str {
        match self {
            Self::Tamam => "tamam",
            Self::Kismi => "kısmi",
            Self::Devam => "devam",
            Self::Bekliyor => "bekliyor",
            Self::Ertelendi => "ertelendi",
            Self::Bilinmiyor => "okunamadı",
        }
    }
}

/// Tek bir hedef satırı, deponun sözlüğünden bağımsız.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hedef {
    pub id: String,
    pub ad: String,
    pub durum: Durum,
    /// Deponun yazdığı ham durum sözcüğü (çeviri denetlenebilsin diye saklanır).
    pub ham_durum: Option<String>,
    /// Kanıt / not / gerekçe: hangisi varsa.
    pub not: Option<String>,
}

/// Ortak şemanın etiketi. Depo kendi beyanına bunu yazar (bkz. `docs/HEDEF_SEMASI.md`).
pub const SEMA_ETIKETI: &str = "archify-hedef/1";

/// Beyan hangi dosyadan okundu.
///
/// A politikası (2026-11-02, Ercan): her depo niyetini **ortak şemayla** yazar.
/// Geçiş bitene kadar eski `feature-list.json` da çeviriciyle okunur; ikisini
/// raporda ayırmak, hangi deponun ortak şemaya geçtiğini gösteren veridir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kaynak {
    /// `.harness/hedefler.json` — depo kendi niyetini ortak şemayla yazmış.
    OrtakSema(PathBuf),
    /// `.harness/feature-list.json` — deponun kendi şeması, çeviriciyle okundu.
    Ceviri(PathBuf),
}

impl Kaynak {
    /// Kısa etiket (rapor için).
    pub fn etiket(&self) -> &'static str {
        match self {
            Self::OrtakSema(_) => "ortak şema",
            Self::Ceviri(_) => "çevirici (eski şema)",
        }
    }

    /// Dosya yolu.
    pub fn yol(&self) -> &Path {
        match self {
            Self::OrtakSema(p) | Self::Ceviri(p) => p,
        }
    }
}

/// Bir deponun amaç beyanı.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tahta {
    pub misyon: Option<String>,
    pub hedefler: Vec<Hedef>,
    /// Beyanda yazılı şema etiketi (`sema` alanı). `None` = eski şema.
    pub sema: Option<String>,
    /// Beyanın okunduğu dosya. `cozumle` (bellekten) için `None`.
    pub kaynak: Option<Kaynak>,
}

/// Okuma hatası. Hepsi "bu depo beyan etmiyor" sınıfındadır, panik değil.
#[derive(Debug, thiserror::Error)]
pub enum Hata {
    #[error("hedef beyanı okunamadı ({yol}): {kaynak}")]
    Okunamadi {
        yol: PathBuf,
        #[source]
        kaynak: std::io::Error,
    },
    #[error(
        "hedef beyanı yok ({dizin}): ne `.harness/hedefler.json` (ortak şema) ne `.harness/feature-list.json` (eski şema) var"
    )]
    BeyanYok { dizin: PathBuf },
    #[error("hedef beyanı geçerli JSON değil ({yol}): {kaynak}")]
    Cozulemedi {
        yol: PathBuf,
        #[source]
        kaynak: serde_json::Error,
    },
    #[error("hedef beyanının kökü nesne değil ({yol})")]
    KokNesneDegil { yol: PathBuf },
    #[error(
        "hedef beyanında hedef dizisi yok ({yol}): beklenen anahtar `features` ya da `hedefler`"
    )]
    HedefDizisiYok { yol: PathBuf },
}

impl Tahta {
    /// Deponun hedef beyanını okur.
    ///
    /// Sıra: önce `.harness/hedefler.json` (ortak şema, A politikası), yoksa
    /// `.harness/feature-list.json` (eski şema, çevirici). Var olan ortak şema
    /// dosyası bozuksa **eski dosyaya düşülmez** — bozuk beyan sessizce eski
    /// veriyle maskelenmemeli.
    pub fn oku(dizin: impl AsRef<Path>) -> Result<Self, Hata> {
        let kok = dizin.as_ref().join(".harness");
        let ortak = kok.join("hedefler.json");
        let eski = kok.join("feature-list.json");

        let (metin, kaynak) = match std::fs::read_to_string(&ortak) {
            Ok(m) => (m, Kaynak::OrtakSema(ortak)),
            Err(k) if k.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::read_to_string(&eski) {
                    Ok(m) => (m, Kaynak::Ceviri(eski)),
                    Err(k2) if k2.kind() == std::io::ErrorKind::NotFound => {
                        return Err(Hata::BeyanYok {
                            dizin: dizin.as_ref().to_path_buf(),
                        })
                    }
                    Err(k2) => {
                        return Err(Hata::Okunamadi {
                            yol: eski,
                            kaynak: k2,
                        })
                    }
                }
            }
            Err(k) => {
                return Err(Hata::Okunamadi {
                    yol: ortak,
                    kaynak: k,
                })
            }
        };

        let mut tahta = Self::cozumle_beyan(&metin, kaynak.yol())?;
        tahta.kaynak = Some(kaynak);
        Ok(tahta)
    }

    /// Beyan metnini çözer. `yol` yalnız hata iletisinde görünür.
    pub fn cozumle(beyan: &str) -> Result<Self, Hata> {
        Self::cozumle_beyan(beyan, Path::new("<bellek>"))
    }

    fn cozumle_beyan(beyan: &str, yol: &Path) -> Result<Self, Hata> {
        let kok: serde_json::Value =
            serde_json::from_str(beyan).map_err(|kaynak| Hata::Cozulemedi {
                yol: yol.to_path_buf(),
                kaynak,
            })?;
        let nesne = kok.as_object().ok_or_else(|| Hata::KokNesneDegil {
            yol: yol.to_path_buf(),
        })?;

        let metin_alan = |adlar: &[&str]| -> Option<String> {
            adlar
                .iter()
                .find_map(|a| nesne.get(*a).and_then(|d| d.as_str()))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let misyon = metin_alan(&["mission", "misyon", "gorev", "görev", "plan"]);
        let sema = metin_alan(&["sema", "şema", "schema"]);
        let dizi = nesne
            .get("features")
            .or_else(|| nesne.get("hedefler"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Hata::HedefDizisiYok {
                yol: yol.to_path_buf(),
            })?;

        let mut hedefler = Vec::with_capacity(dizi.len());
        for (sira, oge) in dizi.iter().enumerate() {
            let Some(o) = oge.as_object() else {
                // Dizi içinde nesne olmayan satır: uydurma yok, okunamadı diye kayda geçer.
                hedefler.push(Hedef {
                    id: format!("?{}", sira + 1),
                    ad: String::from("<nesne değil>"),
                    durum: Durum::Bilinmiyor,
                    ham_durum: None,
                    not: None,
                });
                continue;
            };
            let al = |adlar: &[&str]| -> Option<String> {
                adlar
                    .iter()
                    .find_map(|a| o.get(*a).and_then(|d| d.as_str()))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            };
            let ham_durum = al(&["status", "durum"]);
            let durum = ham_durum
                .as_deref()
                .map(Durum::cevir)
                .unwrap_or(Durum::Bilinmiyor);
            hedefler.push(Hedef {
                id: al(&["id"]).unwrap_or_else(|| format!("?{}", sira + 1)),
                ad: al(&["name", "title", "baslik", "başlık", "goal", "ad"])
                    .unwrap_or_else(|| String::from("<ad yok>")),
                durum,
                ham_durum,
                not: al(&[
                    "evidence",
                    "kanit",
                    "kanıt",
                    "notes",
                    "not",
                    "limitation",
                    "reason",
                    "neden",
                    "engel",
                ]),
            });
        }

        Ok(Self {
            misyon,
            hedefler,
            sema,
            kaynak: None,
        })
    }

    /// Durum başına sayı. Sıra `Durum`'un tanım sırasıdır.
    pub fn sayim(&self) -> [usize; 6] {
        let mut s = [0usize; 6];
        for h in &self.hedefler {
            s[h.durum as usize] += 1;
        }
        s
    }

    /// Bütün satırlar tanındı ve hepsi `Tamam` mı? Tahtanın tek yeşil ölçütü budur;
    /// eksik veri yeşil sayılmaz. Tanınmayan **şema etiketi** de yeşili bozar:
    /// sürümü bilinmeyen bir dosyanın satırlarını "tamam" saymak tahmin olurdu.
    pub fn tamamen_tamam(&self) -> bool {
        !self.hedefler.is_empty()
            && self.sema_gecerli()
            && self.sayim()[Durum::Tamam as usize] == self.hedefler.len()
    }

    /// Şema etiketi ya hiç yazılmamış (eski şema, çevirici okur) ya da bizimki.
    pub fn sema_gecerli(&self) -> bool {
        self.sema.as_deref().is_none_or(|s| s == SEMA_ETIKETI)
    }

    /// Okunamayan satırlar: tanınmayan durum sözcüğü ya da hiç durum yazılmamış.
    pub fn okunamayanlar(&self) -> impl Iterator<Item = &Hedef> {
        self.hedefler
            .iter()
            .filter(|h| h.durum == Durum::Bilinmiyor)
    }

    /// Tek satırlık sayım özeti. CLI'nin `--ozet` çıktısı ile MCP aracının özeti
    /// aynı biçimi kullansın diye burada durur.
    pub fn ozet_satiri(&self) -> String {
        let s = self.sayim();
        format!(
            "{} hedef · {} tamam · {} kısmi · {} devam · {} bekliyor · {} ertelendi · {} okunamadı",
            self.hedefler.len(),
            s[0],
            s[1],
            s[2],
            s[3],
            s[4],
            s[5]
        )
    }

    /// İnsan için tahta metni. `kaynak` yalnız başlıkta görünür.
    ///
    /// Satırlar `format!` + `push_str` ile kurulur: `writeln!` bir `Result`
    /// döndürür ve `String`'e yazarken o `Result` her zaman `Ok`tur — `let _ =`
    /// ile susturmak ya da `expect` ile "hata olamaz" demek yerine hata yolu
    /// hiç doğmuyor.
    pub fn rapor(&self, kaynak: &str) -> String {
        let mut c = String::new();
        let s = self.sayim();
        c.push_str(&format!("amaç tahtası — {kaynak}\n"));
        if let Some(k) = &self.kaynak {
            c.push_str(&format!("kaynak: {} — {}\n", k.yol().display(), k.etiket()));
        }
        if let Some(sema) = &self.sema {
            if self.sema_gecerli() {
                c.push_str(&format!("şema: {sema}\n"));
            } else {
                c.push_str(&format!(
                    "şema: {sema} — TANINMIYOR (beklenen {SEMA_ETIKETI}); tahta yeşil sayılmaz\n"
                ));
            }
        }
        if let Some(m) = &self.misyon {
            c.push_str(&format!("misyon: {m}\n"));
        }
        c.push_str(&format!(
            "hedefler: {} ({} {} · {} {} · {} {} · {} {} · {} {} · {} {})\n",
            self.hedefler.len(),
            s[0],
            Durum::Tamam.etiket(),
            s[1],
            Durum::Kismi.etiket(),
            s[2],
            Durum::Devam.etiket(),
            s[3],
            Durum::Bekliyor.etiket(),
            s[4],
            Durum::Ertelendi.etiket(),
            s[5],
            Durum::Bilinmiyor.etiket(),
        ));
        c.push('\n');
        for h in &self.hedefler {
            c.push_str(&format!("  {} {:<6} {}\n", h.durum.isaret(), h.id, h.ad));
        }
        let okunamayan: Vec<&Hedef> = self.okunamayanlar().collect();
        if !okunamayan.is_empty() {
            c.push('\n');
            c.push_str(&format!(
                "okunamadı — {} satır yeşil sayılmadı:\n",
                okunamayan.len()
            ));
            for h in okunamayan {
                let ham = h.ham_durum.as_deref().unwrap_or("<durum yok>");
                c.push_str(&format!("  {} {:<6} durum sözcüğü: {ham}\n", h.id, h.ad));
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nazar'ın şeması: `name` + `evidence`, durum sözcüğü `passing`.
    const NAZAR: &str = r#"{
      "mission": "altın küme",
      "gate": ["cargo test"],
      "features": [
        {"id":"F1","name":"sorgu üreticisi","status":"passing","evidence":"300 satır"},
        {"id":"F2","name":"ölçüt çekirdeği","status":"passing","evidence":"9 test"}
      ]
    }"#;

    /// altın-kapi'nın şeması: `title` + `verification_contract`, durumlar `done`/`partial`.
    const KAPI: &str = r#"{
      "gate": ["x"],
      "baseline": {},
      "features": [
        {"id":"B1","title":"bellek","status":"done","verification_contract":"unsafe tabanı"},
        {"id":"B2","title":"önyüz","status":"partial","limitation":"ölçülmedi"}
      ]
    }"#;

    /// okf'nin şeması: Türkçe anahtarlar ve `hedefler` dizisi.
    const OKF: &str = r#"{
      "misyon": "Keep notları",
      "hedefler": [
        {"id":"H-1","ad":"hassas tespiti","durum":"done"},
        {"id":"E-1","ad":"düşük öncelikli","durum":"deferred","neden":"tür sınırı doldu"}
      ]
    }"#;

    #[test]
    fn ingilizce_sema_okunur() {
        let t = Tahta::cozumle(NAZAR).expect("çözülmeli");
        assert_eq!(t.misyon.as_deref(), Some("altın küme"));
        assert_eq!(t.hedefler.len(), 2);
        assert_eq!(t.hedefler[0].ad, "sorgu üreticisi");
        assert_eq!(t.hedefler[0].durum, Durum::Tamam);
        assert_eq!(t.hedefler[0].not.as_deref(), Some("300 satır"));
        assert!(t.tamamen_tamam());
    }

    #[test]
    fn kismi_durum_tamami_bozar() {
        let t = Tahta::cozumle(KAPI).expect("çözülmeli");
        assert_eq!(t.hedefler[1].durum, Durum::Kismi);
        assert_eq!(
            t.hedefler[1].not.as_deref(),
            Some("ölçülmedi"),
            "limitation alanı kanıt yerine geçer"
        );
        assert!(!t.tamamen_tamam(), "kısmi satır varken tahta yeşil değil");
        assert_eq!(t.sayim(), [1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn turkce_anahtarlar_okunur() {
        let t = Tahta::cozumle(OKF).expect("çözülmeli");
        assert_eq!(t.misyon.as_deref(), Some("Keep notları"));
        assert_eq!(t.hedefler[0].ad, "hassas tespiti");
        assert_eq!(t.hedefler[1].durum, Durum::Ertelendi);
        assert_eq!(t.hedefler[1].not.as_deref(), Some("tür sınırı doldu"));
    }

    #[test]
    fn taninmayan_durum_yesil_sayilmaz() {
        let t = Tahta::cozumle(
            r#"{"features":[{"id":"X","name":"bilinmez","status":"kafamda_bitti"}]}"#,
        )
        .expect("çözülmeli");
        assert_eq!(t.hedefler[0].durum, Durum::Bilinmiyor);
        assert_eq!(t.hedefler[0].ham_durum.as_deref(), Some("kafamda_bitti"));
        assert!(!t.tamamen_tamam());
        assert_eq!(t.okunamayanlar().count(), 1);
        let r = t.rapor("deneme");
        assert!(r.contains("okunamadı — 1 satır yeşil sayılmadı"), "{r}");
        assert!(
            r.contains("kafamda_bitti"),
            "ham sözcük raporda görünmeli: {r}"
        );
    }

    #[test]
    fn durum_yoksa_okunamadi_olur() {
        let t =
            Tahta::cozumle(r#"{"features":[{"id":"X","name":"durumsuz"}]}"#).expect("çözülmeli");
        assert_eq!(t.hedefler[0].durum, Durum::Bilinmiyor);
        assert_eq!(t.hedefler[0].ham_durum, None);
        assert!(t.rapor("d").contains("<durum yok>"));
    }

    #[test]
    fn bos_dizi_tamamen_tamam_degil() {
        let t = Tahta::cozumle(r#"{"features":[]}"#).expect("çözülmeli");
        assert!(!t.tamamen_tamam(), "boş tahta yeşil değil");
        assert_eq!(t.sayim(), [0; 6]);
    }

    #[test]
    fn dizi_icinde_nesne_olmayan_satir_kaybolmaz() {
        let t = Tahta::cozumle(r#"{"features":["metin satırı"]}"#).expect("çözülmeli");
        assert_eq!(t.hedefler.len(), 1);
        assert_eq!(t.hedefler[0].durum, Durum::Bilinmiyor);
        assert_eq!(t.hedefler[0].id, "?1");
    }

    #[test]
    fn hedef_dizisi_yoksa_hata() {
        let h = Tahta::cozumle(r#"{"mission":"var ama özellik yok"}"#).unwrap_err();
        assert!(matches!(h, Hata::HedefDizisiYok { .. }), "{h}");
    }

    #[test]
    fn kok_dizi_ise_hata() {
        let h = Tahta::cozumle("[]").unwrap_err();
        assert!(matches!(h, Hata::KokNesneDegil { .. }), "{h}");
    }

    #[test]
    fn bozuk_json_hata_doner_panik_yok() {
        let h = Tahta::cozumle("{").unwrap_err();
        assert!(matches!(h, Hata::Cozulemedi { .. }), "{h}");
    }

    #[test]
    fn durum_sozcukleri_cevrilir() {
        for (s, d) in [
            ("passing", Durum::Tamam),
            ("IN_PROGRESS", Durum::Devam),
            ("deferred", Durum::Ertelendi),
            ("  Pending ", Durum::Bekliyor),
            ("partial", Durum::Kismi),
            ("tamam", Durum::Tamam),
            ("", Durum::Bilinmiyor),
        ] {
            assert_eq!(Durum::cevir(s), d, "{s:?}");
        }
    }

    #[test]
    fn sayim_durum_sirasina_uyar() {
        let t = Tahta::cozumle(
            r#"{"features":[
              {"id":"a","name":"a","status":"deferred"},
              {"id":"b","name":"b","status":"done"},
              {"id":"c","name":"c","status":"in_progress"}
            ]}"#,
        )
        .expect("çözülmeli");
        assert_eq!(t.sayim(), [1, 0, 1, 0, 1, 0]);
        assert!(t.rapor("x").contains("hedefler: 3 (1 tamam"));
    }

    /// A politikası: depo niyetini `.harness/hedefler.json` ile ortak şemada
    /// yazar. Bu dosya eski `feature-list.json`'ı **gölgeler** — yoksa aynı depo
    /// iki gerçeği taşır ve hangisinin geçerli olduğu tahmine kalır.
    #[test]
    fn ortak_sema_dosyasi_eski_dosyayi_golgede_birakir() {
        let dizin = gecici("oncelik");
        let harness = dizin.join(".harness");
        std::fs::create_dir_all(&harness).unwrap();
        yaz(
            &harness.join("feature-list.json"),
            r#"{"features":[{"id":"ESKI","name":"eski","status":"pending"}]}"#,
        );
        yaz(
            &harness.join("hedefler.json"),
            r#"{"sema":"archify-hedef/1","misyon":"yeni","hedefler":[
                 {"id":"H-1","ad":"ortak","durum":"tamam","kanit":"ölçüldü"}
               ]}"#,
        );

        let t = Tahta::oku(&dizin).expect("okunmalı");
        assert_eq!(t.hedefler.len(), 1, "eski dosyadan satır sızmamalı");
        assert_eq!(t.hedefler[0].id, "H-1");
        assert_eq!(t.sema.as_deref(), Some(SEMA_ETIKETI));
        assert!(matches!(t.kaynak, Some(Kaynak::OrtakSema(_))));
        assert!(t.tamamen_tamam());
        let r = t.rapor("depo");
        assert!(r.contains("ortak şema"), "kaynak raporda görünmeli: {r}");

        std::fs::remove_dir_all(&dizin).ok();
    }

    /// Geçiş bitmemiş depo: yalnız eski dosya var. Tahta onu okumalı ama
    /// kaynağı "çevirici" diye **işaretlemeli** — eksikliği gösteren veri budur.
    #[test]
    fn eski_sema_cevirici_diye_isaretlenir() {
        let dizin = gecici("ceviri");
        let harness = dizin.join(".harness");
        std::fs::create_dir_all(&harness).unwrap();
        yaz(
            &harness.join("feature-list.json"),
            r#"{"features":[{"id":"F1","name":"eski","status":"passing"}]}"#,
        );

        let t = Tahta::oku(&dizin).expect("okunmalı");
        assert!(matches!(t.kaynak, Some(Kaynak::Ceviri(_))));
        assert!(t.rapor("depo").contains("çevirici"));
        assert!(t.sema.is_none(), "eski şemada etiket yok; uydurulmamalı");

        std::fs::remove_dir_all(&dizin).ok();
    }

    /// Bozuk ortak şema eski dosyaya düşmemeli: yoksa bozuk beyan sessizce
    /// eski veriyle maskelenir ve tahta yanlış şeyi "güncel" sanır.
    #[test]
    fn bozuk_ortak_sema_eski_dosyaya_dusmez() {
        let dizin = gecici("bozuk");
        let harness = dizin.join(".harness");
        std::fs::create_dir_all(&harness).unwrap();
        yaz(
            &harness.join("feature-list.json"),
            r#"{"features":[{"id":"F1","name":"eski","status":"passing"}]}"#,
        );
        yaz(&harness.join("hedefler.json"), "{\"sema\": ");

        let h = Tahta::oku(&dizin).unwrap_err();
        assert!(matches!(h, Hata::Cozulemedi { .. }), "{h}");

        std::fs::remove_dir_all(&dizin).ok();
    }

    /// Hiç beyan yok: hata iki yolu da adıyla söylemeli.
    #[test]
    fn beyan_yoksa_iki_yol_da_adlandirilir() {
        let dizin = gecici("yok");
        std::fs::create_dir_all(dizin.join(".harness")).unwrap();
        let h = Tahta::oku(&dizin).unwrap_err();
        let m = h.to_string();
        assert!(m.contains("hedefler.json"), "{m}");
        assert!(m.contains("feature-list.json"), "{m}");
        std::fs::remove_dir_all(&dizin).ok();
    }

    /// Tanınmayan şema etiketi: satırlar okunur ama tahta yeşil sayılmaz.
    #[test]
    fn taninmayan_sema_etiketi_yesili_bozar() {
        let t = Tahta::cozumle(
            r#"{"sema":"archify-hedef/99","hedefler":[{"id":"H-1","ad":"a","durum":"tamam"}]}"#,
        )
        .expect("çözülmeli");
        assert_eq!(t.sayim()[0], 1, "satır tamam olarak okunur");
        assert!(!t.tamamen_tamam(), "sürümü bilinmeyen tahta yeşil değil");
        let r = t.rapor("x");
        assert!(r.contains("TANINMIYOR"), "{r}");
    }

    fn gecici(ad: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("agr-hedef-{ad}-{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn yaz(yol: &Path, metin: &str) {
        std::fs::write(yol, metin).unwrap();
    }
}
