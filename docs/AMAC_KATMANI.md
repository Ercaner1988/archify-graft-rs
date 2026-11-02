# Amaç katmanı: tasarım notu ve ölçülmüş zemin (2026-11-02)

Bu not `docs/VIZYON.md`'nin iki açık sorusuna karar vermek için değil, **kararı
verilebilir hâle getirmek** için yazıldı. Bütün sayılar bu makinede koşulmuş
komutların çıktısıdır; uydurulmuş eşik, ağırlık ya da oran yoktur.

## 1. Bugünkü durum: Graft var, amaç yok

Depoda "hedef", "beklenti", "değerlendirme tahtası" kavramını taşıyan hiçbir
düğüm, crate ya da alan yok. `NodeKind` yalnız koddan türeyenleri tanır:
`File, Function, Method, Class, Interface, TypeAlias, Enum, Module, Constant,
Crate, ExternalCrate`. Yani **Graft zemin katmanı ölçüyor; Archify'ın amaç
katmanı henüz yok.**

Zemin katmanının bugünkü çıktısı (kendi ikilimizle, iki ayrı ağaçta):

| Ağaç | dosya | sembol | kenar | indeksleme | önbellek |
|---|---|---|---|---|---|
| Nazar (görsel dal birleşmiş hâli) | 166 | 1489 | 2681 | 0,2 sn | 2,65 MB |
| archify-graft-rs (kendi deposu) | 139 | 1089 | 2069 | artımlı | 1,69 MB |

Bu iki sayı aynı komutun iki farklı depodaki davranışıdır; karşılaştırma
değil, ölçek göstergesidir.

## 2. Birinci açık soru: tahtanın ölçütü nereden gelir?

Aday üç kaynak ölçüldü.

### 2.1 `.harness/feature-list.json` — makine okunur, ama ortak şeması yok

Sekiz depoda bulundu. Hepsi aynı adı taşıyor, **hiçbiri aynı şemayı taşımıyor**:

| Depo | kök anahtarlar | özellik | durum sözlüğü | öğe alanları |
|---|---|---|---|---|
| Nazar | mission, created, gate, features | 8 | passing | id, name, status, evidence |
| altin-kapi | gate, baseline, kisitlar, features | 17 | done, partial | id, title, status, verification_contract, evidence, limitation |
| el-Fihrist | gorev, verification_contract, features | 6 | done | id, repo, baslik, status, kanit, neden |
| kesfuzzunun | mission, canonical_gate, verification_contract, baseline, features | 11 | done, pending | id, title, status, notes, verification_contract |
| logseq-tepsi | gate, verification_contract, features | 14 | done, deferred | id, title, status, reason |
| okf | misyon, kararlar, dogrulama_sozlesmesi, kapi_komutu, **hedefler** | 0 | — | — (farklı ad) |
| pasli-beyin | plan, gate, verification_contract, features | 8 | done, in_progress, pending | id, goal, status |
| wbc-rs | gate, verification_contract, features | 27 | in_progress, deferred, done, pending | id, goal, status, reason, ad, neden, kaynak |

Ölçülen sonuç: **91 özellik satırı, 3 farklı adlandırma dili (name/title/baslik/
goal/ad), 7 farklı durum sözcüğü.** Tahta bu dosyaları doğrudan okursa her depo
için ayrı ayrıştırıcı yazmak gerekir; bu, tahtayı kırılgan yapar.

### 2.2 Altın kapı — ölçüyor, ama makine okumuyor

`kapi` boyutları yeşil/kırmızı/ölçülmedi diye hükme bağlıyor ve çıkış kodu
veriyor (0 yeşil, 1 kırmızı ya da ölçülmedi). **JSON çıktısı yok**; çıktı
insan içindir. Tahta kapıyı doğrudan tüketemez, metin ayrıştırmak zorunda kalır.

İki depoda bugünkü hüküm:

| Depo | yeşil | kırmızı | ölçülmedi | uygulanmaz | çıkış |
|---|---|---|---|---|---|
| Nazar (görsel dal birleşmiş hâli) | 6 | 0 | 9 | 2 | 1 |
| archify-graft-rs | 9 | 0 | 9 | 2 | 0 |

`kapi` çıkış kodu, "ölçülmedi" varken de 1 döner. Tahta için bu ayrım
önemlidir: "kırmızı" ile "bakılmadı" aynı şey değildir, aynı renge boyanamaz.

### 2.3 Sınama sayıları — hazır ve sayısal

`cargo test` çıktısı sayılabilir: Nazar'ın birleşme sonrası durumu 350 geçti,
0 kaldı, 12 yok sayıldı. Bu, tahtanın en ucuz ve en tartışmasız kolonudur.

### 2.4 Sonuç: üç kaynak da tek başına yetmiyor

- `feature-list.json` niyeti taşır (ne istiyoruz), doğrulamayı değil.
- `kapi` doğrulamayı taşır (neye uyuyoruz), niyeti değil; ayrıca makine okumaz.
- Sınama sayısı yalnız "derleniyor ve geçiyor" der; amaca yaklaşmayı söylemez.

Yani tahta, "niyet × doğrulama × sınama" üçlüsünü **birleştiren** bir katman
olmak zorunda; üçünden birini seçmek tahtayı eksik bırakır.

## 3. İkinci açık soru: Graft hafızası el-Fihrist / OKF ile ilişkisi

Logseq'teki vizyon bloğu bu soruyu Ercan adına zaten cevaplıyor: **"OKF, Graft
tarafıyla tamamen birleşecek."** Yani OKF ayrı bir hafıza olmayacak.

Geriye el-Fihrist oturum hafızası kalıyor ve o **başka bir tür**: oturumların
anlattığı olaylar (ne yapıldı, nerede kalındı), kodun yapısı değil. Graft ise
koddan türeyen düğüm ve kenarları tutuyor. İkisi aynı zemine yazılırsa
"fonksiyon" ile "dün ne yaptım" aynı grafikte karışır. Ayrı tutulmaları,
farklı ömür ve farklı doğruluk kuralları gerektirmelerinden gelir.

## 4. Karar gerektiren tek nokta

Tahtanın çekirdeği **normalize edilmiş tek bir özellik kaydı** olmalı: her
depo kendi dosyasını bu kayda çevirir, tahta yalnız kaydı bilir. Böylece
depoların şeması değişse bile tahta kırılmaz.

Geriye kalan soru şudur ve kararı Ercan'da:

**Normalize kayıt nerede doğar?**

- **A. Depo tarafı (beyan):** her depo `.harness/feature-list.json` yanında
  ortak şemalı bir dosya tutar; tahta yalnız onu okur. Depo kendi niyetini
  yazar, tahta hiçbir zaman tahmin etmez. Bedeli: sekiz depoda bir kez yazım
  işi.
- **B. Tahta tarafı (çevirici):** tahta bilinen şemaları tanır, bilinmeyeni
  "tanınmadı" diye işaretler. Depolara dokunulmaz. Bedeli: her yeni şema
  tahtada kod değişikliği ister ve sessiz yanlış okuma riski doğar.
- **C. Karma:** tahta çeviriciyle okur, ama tanımadığı satırı **yeşil saymaz**;
  "okunamadı" olarak gösterir ve depo tarafına uyarı bırakır. A'nın sağlamlığı,
  B'nin dokunmazlığı; bedeli iki yerde bakım.

Öneri: **C**. Gerekçe ölçülmüş bir olguya dayanıyor — aynı adı taşıyan sekiz
dosyanın hiçbiri aynı şemada değil, yani "şema sabit" varsayımı bugün yanlış;
ama "her depoyu şimdi düzeltelim" demek de tahtayı aylarca bekletir. C, bugün
çalışır ve yanlış okumayı görünür kılar (kural: *ölçülmedi asla yeşil
sayılmaz* — `kapi`nın kendi ilkesi).

**Karar (Ercan, 2026-11-02): A.** Depo tarafı. Her depo niyetini ortak şemayla
yazar (`.harness/hedefler.json`, etiket `archify-hedef/1`); tahta tahmin etmez.
Sözleşme: `docs/HEDEF_SEMASI.md`.

**Uygulama (2026-11-02):**

- `5d2434e` — çevirici: `archify-hedef` crate'i + `hedef` alt komutu + `graft_hedef`
  MCP aracı. Okunamayan satır `okunamadı` olur, çıkış kodu 1 / `isError`.
- bu commit — A: tahta önce `.harness/hedefler.json` okur, yalnız yoksa eski
  `feature-list.json`'a düşer ve onu rapor `çevirici (eski şema)` diye
  **işaretler**. Bozuk ortak şema eskiye düşmez (maskelenme yasak). Tanınmayan
  şema etiketi tahtayı yeşil saydırmaz. `archify-graft-rs` kendi beyanını yazdı:
  10 hedef — 5 tamam, 1 devam (depoların geçişi), 3 bekliyor, 1 ertelendi.

Geçiş durumu bugün **1/9 depo** (bu depo). Diğer sekiz depo hâlâ eski şemada ve
tahta onları çevirici diye işaretliyor — eksikliği gösteren veri budur, gizlenmiyor.

## 5. Bu notun sınırı

- Tahtanın görsel biçimi (panel mi, ayrı pencere mi, egui stüdyosunun içinde mi)
  bu notta tartışılmadı; o, kayıt şeması kesinleştikten sonraki adım.
- `kapi`ya JSON çıktısı eklemek ayrı bir iş; bu not yalnız bugünkü durumu
  kaydediyor.
- Hiçbir eşik, ağırlık ya da "yeterlilik" oranı önerilmedi: onlar ancak gerçek
  özellik listeleri normalize edildikten sonra anlamlı olur.
