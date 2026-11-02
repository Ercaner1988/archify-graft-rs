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
işidir. Tahta bugün 1/9 gösteriyor; bu bir eksiklik kaydıdır, gizlenmiyor.

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

## Bugünkü durum (2026-11-02)

| Depo | `.harness/hedefler.json` | `.harness/feature-list.json` |
|---|---|---|
| archify-graft-rs | **var** (10 hedef) | — |
| altin-kapi | yok | var |
| el-Fihrist | yok | var |
| kesfuzzunun | yok | var |
| logseq-tepsi | yok | var |
| Nazar | yok | var |
| okf | yok | var |
| pasli-beyin | yok | var |
| wbc-rs | yok | var |

`kunye` ve `oturum-ozeti` depolari hiç beyan taşımıyor — onlar için geçiş
değil, ilk yazım söz konusu.

## Sıra önerisi

En çok kullanılan zeminden başla: `altin-kapi` (kapı her depoda koşuyor),
sonra `Nazar`, `el-Fihrist`, `kesfuzzunun`. Kalanlar kendi temposunda.
Sıra bağlayıcı değil; tahta hangi deponun geçtiğini kendisi gösterir.
