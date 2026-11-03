# Ortak şemaya geçiş — depo kendi niyetini nasıl yazar

Bu dosya **süreç**tir; şemanın kendisi `HEDEF_SEMASI.md`'de.

Karar: **A — depo tarafı** (Ercan, 2026-11-02). Her depo kendi niyetini kendi
`.harness/hedefler.json` dosyasına yazar. Tahta tahmin etmez, çevirmez.

## Neden merkezî çeviri yapılmıyor

Sekiz deponun `feature-list.json` dosyasını bir betikle ortak şemaya çevirmek
mümkün — ve yanlış. Şema döner ama **niyet yazılmış olmaz**: hangi hedefin
gerçekten bittiği, hangisinin ölçülmediği, hangisinin bilerek ertelendiği o
deponun kendi bilgisidir. Merkezî çeviri, "ölçülmeyeni yeşil sayma" ilkesini
daha ilk günde bozar: eski dosyadaki `status: passing` satırları kanıtsız
"tamam" olarak kopyalanır.

Bu yüzden geçiş **depo başına bir yazım işi**dir ve o depoyu çalışan oturumun
işidir. Tahta bugün 9/11 gösteriyor; kalan iki depo bu bir eksiklik kaydıdır,
gizlenmiyor.

## Kime iletilir

Depoyu çalışan ajana/oturuma. Ercan'ın kendisinin yapacağı bir iş yok: her
depoda zaten bir oturum açılıyor, o oturuma aşağıdaki paragraf yapıştırılıyor.

## Yapıştırılacak istem

> Bu depo, Archify amaç tahtasının ortak şemasına (`archify-hedef/1`) geçecek.
> `.harness/feature-list.json` dosyasını **mekanik olarak çevirme**; içeriğini
> oku ve `.harness/hedefler.json` dosyasını kendi niyetinle yeniden yaz.
> Sözleşme: `archify-graft-rs/docs/HEDEF_SEMASI.md`. Kurallar:
> (1) her satır gerçekten ölçülmüşse `tamam` + `kanit` alanına ölçümü yaz
> (komut, sayı, dosya); (2) ölçmediğin şeyi `tamam` yazma — `bekliyor` ya da
> `devam` yaz, `engel` alanına takıldığı yeri yaz; (3) bilerek vazgeçtiysen
> `ertelendi` + gerekçe; (4) `misyon` tek cümle olsun; (5) eski dosyayı silme,
> gölgede kalır. Sonra `archify-graft-cli hedef .` koş ve çıktıyı rapora koy.

Doğrulama (her depoda aynı):

```
archify-graft-cli hedef <depo yolu>
archify-graft-cli hedef <depo yolu> --ozet
```

Çıktıda `kaynak: … — ortak şema` görünüyorsa geçiş tamam; `çevirici (eski şema)`
görünüyorsa o depo henüz yazmamış.

## Bugünkü durum (2026-11-03)

Aşağıdaki satırlar elle yazılmadı: `archify-graft-cli hedef <depo> --ozet` ve
`archify-graft-cli hedef <depo>` (kaynak satırı) çıktısından dolduruldu. Sütun 3
tahtanın kendi hükmüdür — tablodaki 11 depodan **9'u** ortak şemada, 2'si
henüz değil.

| Depo | Tahta (ortak şema) | Eski `feature-list.json` | Durum |
|---|---|---|---|
| archify-graft-rs | **var** — 10 hedef · 8 tamam · 1 devam · 1 bekliyor | — | geçti |
| altin-kapi | **var** — 10 hedef · 4 tamam · 1 kısmi · 1 devam · 4 bekliyor | var (gölgede) | geçti (03.11) |
| el-Fihrist | **var** — 13 hedef · 5 tamam · 3 kısmi · 5 bekliyor | var (gölgede) | geçti |
| kesfuzzunun | **var** — 13 hedef · 9 tamam · 1 kısmi · 3 bekliyor | var (gölgede) | geçti (03.11) |
| logseq-tepsi | **var** — 15 hedef · 7 tamam · 7 kısmi · 1 bekliyor | var (gölgede) | geçti |
| Nazar | **var** — 9 hedef · 6 tamam · 1 kısmi · 2 devam | var (gölgede) | geçti |
| oturum-ozeti | **var** — 6 hedef · 4 tamam · 1 kısmi · 1 bekliyor | — | geçti (hiç beyanı yoktu) |
| wbc-rs | **var** — 29 hedef · 4 tamam · 18 kısmi · 1 bekliyor · 6 ertelendi | var (gölgede) | geçti (03.11) |
| kunye | **var** — 13 hedef · 10 tamam · 1 kısmi · 2 bekliyor | yok | geçti (03.11, ilk yazım) |
| okf | yok | var — 13 hedef (7 tamam · 6 ertelendi) | **kaldı** |
| pasli-beyin | yok | var — 8 hedef (8 tamam) | **kaldı** |

`okf` ve `pasli-beyin` bu tabloya göre "kaldı" görünüyor ama **bu oturumda
dokunulmadı**: ikisinde de etkin çalışan bir oturum var, istem Ercan tarafından
iletilecek.

### `wbc-rs` — neden 18 satır "kısmi"

Tahta ölçümü 2026-11-03'te sandık sandık koşuldu (`cargo test -p <sandık>`):
37 sandığın 28'i yeşil, 632 sınama geçti / 0 düştü. Yedi sandık derlenmiyor
(`wbc-panel`, `wbc-maiyet`, `wbc-gorunum`, `wbc-ayrinti`, `wbc-hane`,
`wbc-komuta`, `wbc-kumanda`), iki sandıkta birer sınama düşüyor
(`wbc-tarayici`, `wbc-sahne`). Çalışma ağacı commit'siz. Bu yüzden çoğu eski
`in_progress` satırı `tamam`a yuvarlanmadı: niyet korundu, kanıt ölçümün
gösterdiği yere kadar yazıldı, engel alanına tam derleme hatası ve dosya:satır
kondu. `cargo fmt --all --check` de kırmızı (`crates/wbc-app/src/main.rs:485`).

### Uyarı — "geçti" hükmü bugün yalnız bu makinede geçerli

2026-11-03'te `git check-ignore` ile ölçüldü: `.harness/hedefler.json` sekiz
depodan **yedisinde `.gitignore` ile yok sayılıyor**; izlenen iki depo
`archify-graft-rs` ve `kunye`. Yani el-Fihrist, kesfuzzunun, altin-kapi, Nazar,
logseq-tepsi, oturum-ozeti ve **wbc-rs** tahtaları **yalnız yerel diskte** var —
klonlayan, CI koşucusu ya da başka makinedeki ajan onları görmez.

Bu bir kusur mu, tercih mi, ölçülmeden karar verilmez: `.harness/` bu depolarda
bilerek yok sayılıyor olabilir (çalışma defteri, ürün değil). Ama o hâlde
"geçiş tamam" cümlesi depoyla taşınmıyor demektir ve tabloda bunun yazması
gerekir. İzlenmesi isteniyorsa her depo önce kendi G0 sızıntı kapısından
geçirmek zorunda: bugünkü tahtalarda kişisel mutlak yollar var
(`C:\Users\buzbe\…`), yani `!/​.harness/hedefler.json` istisnası yazmak tek
başına kapıyı kırmızıya çevirir. Karar Ercan'ın.


