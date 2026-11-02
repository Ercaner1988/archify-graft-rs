# Hedef şeması — `archify-hedef/1`

Bir depo ne yapmaya çalıştığını **makine okunur** biçimde beyan eder. Bu dosya
sözleşmedir; `archify-hedef` crate'i, `hedef` alt komutu ve `graft_hedef` MCP
aracı bu sözleşmeyi okur.

Karar: **A — depo tarafı** (Ercan, 2026-11-02). Her depo kendi niyetini bu şemayla
yazar; tahta tahmin etmez. Ölçülen gerekçe: aynı adı taşıyan sekiz
`.harness/feature-list.json` dosyasının hiçbiri aynı şemada değildi — 91 satır, üç
adlandırma dili (`name`/`title`/`baslik`/`goal`/`ad`), yedi durum sözcüğü
(`docs/AMAC_KATMANI.md` §2.1).

## Dosya

`.harness/hedefler.json` — depo kökünde.

Okuma sırası `archify-hedef` içinde sabittir:

1. `.harness/hedefler.json` (bu şema) — varsa **yalnız** bu okunur.
2. `.harness/feature-list.json` (eski, depo başına değişen şema) — yalnız 1. yoksa,
   çeviriciyle okunur ve rapor `çevirici (eski şema)` diye **işaretler**.

Geçiş kuralı: eski dosya silinmek zorunda değil, ama ortak şema dosyası varsa eski
dosya **gölgede kalır**. İki gerçeğin aynı anda geçerli olmasına izin verilmez.

Geçişin kim tarafından, nasıl yapılacağı: `docs/HEDEF_GECIS_TALIMATI.md`.

Bozuk ortak şema dosyası eski dosyaya **düşmez**: bozuk beyan, eski veriyle
maskelenirse tahta yanlış şeyi güncel sanır. Okuma hata verir, çıkış kodu 1 olur.

## Biçim

```json
{
  "sema": "archify-hedef/1",
  "misyon": "Tek cümlelik amaç.",
  "hedefler": [
    {
      "id": "H-1",
      "ad": "Ölçülebilir hedef.",
      "durum": "tamam",
      "kanit": "Nasıl ölçüldüğü (komut, sayı, dosya).",
      "engel": "Yalnız durum bekliyor/ertelendi ise: neyi bekliyor."
    }
  ]
}
```

### Alanlar

| Alan | Zorunlu | Anlam |
|---|---|---|
| `sema` | ✔ | `archify-hedef/1`. Başka bir etiket **tanınmaz**: satırlar okunur ama tahta yeşil sayılmaz. |
| `misyon` | — | Deponun tek cümlelik amacı. |
| `hedefler` | ✔ | Hedef dizisi. Boş dizi geçerlidir ama tahta yeşil değildir. |
| `hedefler[].id` | — | Kısa kimlik (`H-1`). Yoksa `?1`, `?2` diye numaralanır. |
| `hedefler[].ad` | — | Hedef metni. Yoksa `<ad yok>`. |
| `hedefler[].durum` | — | Aşağıdaki sözcüklerden biri. Yoksa ya da tanınmıyorsa **okunamadı**. |
| `hedefler[].kanit` | — | Nasıl ölçüldüğü. `durum: "tamam"` diyen satırın burada bir ölçüm göstermesi beklenir. |
| `hedefler[].engel` | — | Bekleyen/ertelenen hedefin neye takıldığı. |

Eşanlamlı anahtarlar da okunur (eski dosyalarla aynı çevirici): `name`/`title`/
`baslik`/`goal`/`ad`, `status`/`durum`, `evidence`/`kanit`/`notes`/`limitation`/
`reason`/`neden`. Yeni dosyalarda **kanonik** adlar kullanılmalı; eşanlamlılar
yalnız geçiş içindir.

### Durum sözcükleri

| Kanonik | Kabul edilen eşanlamlılar | Anlam |
|---|---|---|
| `tamam` | passing, done, bitti, complete, completed | Bitti; kanıtı `kanit` alanında. |
| `kismi` | partial | Bir kısmı bitti. |
| `devam` | in_progress, in-progress, doing, active | Üzerinde çalışılıyor. |
| `bekliyor` | pending, todo, queued | Sırada, başlanmamış. |
| `ertelendi` | deferred, wontdo, wont_do | Bilerek ertelenmiş; gerekçesi `engel` alanında. |

Katlama Türkçe-güvenlidir (`graft-i18n`'in `fold_tr`'si): `I`, `İ`, `ı` tek biçime
iner, `KISMİ` de tanınır. Tanınmayan sözcük **`tamam`a yuvarlanmaz**.

## Tek yeşil ölçütü

Tahta ancak şu üçü birden doğruysa yeşildir:

1. En az bir hedef var,
2. hiçbir satır okunamadı değil,
3. bütün satırlar `tamam` **ve** şema etiketi tanınıyor.

Eksik veri yeşil sayılmaz; bu `kapi`'nın kendi ilkesidir.

## Çıkış davranışı

- `archify-graft-cli hedef <dizin>`: okunamayan satır varsa **çıkış kodu 1**.
- `archify-graft-cli hedef <dizin> --json`: makine okunur tahta. Alanlar:

  ```json
  {
    "sema": "archify-hedef/1",
    "sema_gecerli": true,
    "kaynak": {"yol": ".harness/hedefler.json", "tur": "ortak-sema", "etiket": "ortak şema"},
    "misyon": "…",
    "yesil": false,
    "sayim": {"toplam": 10, "tamam": 7, "kismi": 0, "devam": 1,
              "bekliyor": 2, "ertelendi": 0, "okunamadi": 0},
    "hedefler": [{"id": "H-1", "ad": "…", "durum": "tamam",
                  "ham_durum": "tamam", "not": "…"}]
  }
  ```

  `kaynak.tur` iki değer alır: `ortak-sema` ya da `cevirici`. `yesil` **tek**
  ölçüttür (`tamamen_tamam`); `sema_gecerli` ayrıca durur ki tüketici "satırlar
  tamam ama şema tanınmıyor" hâlini ayırt edebilsin — o hâlde `yesil` false'tur
  ve sebebi alanda yazılıdır. `--json` ile `--ozet` birlikte verilemez.
- MCP `graft_hedef`: okunamayan satır varsa yanıt `isError: true` ve
  `UYARI: N satır okunamadı — yeşil sayılmadı` metni taşır.
- Beyan hiç yoksa: `hedef beyanı yok (<dizin>): ne .harness/hedefler.json ...
  ne .harness/feature-list.json ...` — hangi dosyaların arandığı adıyla söylenir.

## Örnek

```json
{
  "sema": "archify-hedef/1",
  "misyon": "Ajanlar için ortak kod zeminini ve onun amaç tahtasını sağlamak.",
  "hedefler": [
    {
      "id": "H-1",
      "ad": "Ortak şema okunur",
      "durum": "tamam",
      "kanit": "cargo test -p archify-hedef: 17 sınama"
    },
    {
      "id": "H-2",
      "ad": "Tahtanın görsel paneli",
      "durum": "bekliyor",
      "engel": "Şema kesinleşmeden biçim tartışılmadı (docs/AMAC_KATMANI.md §5)"
    }
  ]
}
```
