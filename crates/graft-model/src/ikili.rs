//! graft'ın kendi yazıp okuduğu dosyaların ikili biçimi (rkyv; ADR altin-kapi 0005).
//!
//! Dosya = 4 bayt imza + 4 bayt biçim sürümü (LE) + 8 bayt boşluk + rkyv yükü. Başlık
//! 16 bayt: bellek eşlemeli dosyada yük 16'ya hizalı kalır (eşleme tabanı sayfa hizalı),
//! bu yüzden kopyalamadan doğrulanabilir; hizasız bir tampondan okunursa hizalı tampona
//! kopyalanır. Sürüm ya da imza tutmayan dosya `None` döner: çağıran onu yok sayıp yeniden
//! üretir (önbellek) ya da "yeniden indeksle" der. Bozuk yük rkyv'nin doğrulamasından
//! (bytecheck) geçemez; güvensiz okuma yok.
//!
//! ponytail: graphify-rs'teki `ikili.rs` ile aynı fikir, ayrı kopya (başlık burada 16
//! bayt). Üçüncü kullanıcıda (pasli-beyin göçü) ortak crate'e çıkarılır.

use rkyv::api::high::{HighDeserializer, HighSerializer, HighValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Error;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;

const IMZA: &[u8; 4] = b"GRFT";
const BASLIK: usize = 16;
/// Model değişince artırılır; eski dosyalar sessizce yeniden üretilir ya da yeniden indekslenir.
pub const SURUM: u32 = 1;

pub fn kodla<T>(deger: &T) -> Result<Vec<u8>, String>
where
    T: for<'a> rkyv::Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
{
    let yuk = rkyv::to_bytes::<Error>(deger).map_err(|e| format!("rkyv kodlama: {e}"))?;
    let mut out = Vec::with_capacity(BASLIK + yuk.len());
    out.extend_from_slice(IMZA);
    out.extend_from_slice(&SURUM.to_le_bytes());
    out.resize(BASLIK, 0);
    out.extend_from_slice(&yuk);
    Ok(out)
}

pub fn coz<T>(bayt: &[u8]) -> Option<T>
where
    T: rkyv::Archive,
    T::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
        + rkyv::Deserialize<T, HighDeserializer<Error>>,
{
    let (bas, yuk) = bayt.split_at_checked(BASLIK)?;
    if &bas[..4] != IMZA || bas[4..8] != SURUM.to_le_bytes() {
        return None;
    }
    if yuk.as_ptr().align_offset(16) == 0 {
        return rkyv::from_bytes::<T, Error>(yuk).ok();
    }
    let mut hizali = AlignedVec::<16>::with_capacity(yuk.len());
    hizali.extend_from_slice(yuk);
    rkyv::from_bytes::<T, Error>(&hizali).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imza_surum_ve_bozulma_denetlenir() {
        let b = kodla(&vec![1u32, 2, 3]).unwrap();
        assert_eq!(coz::<Vec<u32>>(&b), Some(vec![1, 2, 3]));
        let mut eski = b.clone();
        eski[4] = 0;
        assert_eq!(coz::<Vec<u32>>(&eski), None, "eski sürüm yok sayılmalı");
        assert_eq!(
            coz::<Vec<u32>>(b"\x01\x02\x03 eski bincode dosyasi.."),
            None
        );
        assert_eq!(coz::<Vec<u32>>(&b[..9]), None);
        let mut bozuk = b;
        let n = bozuk.len();
        bozuk[n - 1] = 0xFF;
        assert_eq!(
            coz::<Vec<u32>>(&bozuk),
            None,
            "doğrulanmayan yük reddedilmeli"
        );
    }

    #[test]
    fn hizasiz_tampondan_da_okunur() {
        let b = kodla(&vec![7u64, 8, 9]).unwrap();
        // Bir bayt kaydırılmış kopya: yük hizasız başlar.
        let mut kaydirilmis = vec![0u8];
        kaydirilmis.extend_from_slice(&b);
        assert_eq!(coz::<Vec<u64>>(&kaydirilmis[1..]), Some(vec![7, 8, 9]));
    }
}
