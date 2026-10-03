//! `wiring.bin` düzeni: kütüphaneden bağımsız, elle belirtilmiş KÜÇÜK sözleşme.
//!
//! Dosyayı başka araçlar okur (pasli-beyin `kopru.rs` aynı alanları aynı sırayla yansıtır).
//! Repolar arası sözleşme bir serileştirme kitaplığına bağlanmaz: rkyv'nin düzeni tür
//! tanımına ve özelliklere (endian, işaretçi genişliği) bağlıdır, iki repo arasında sabit
//! kalması gereken şey için kırılgandır. Düzen, bincode 2 `standard()`'ın bu yapı için
//! yazdığı baytlarla bayt bayt AYNIDIR, o yüzden mevcut okuyucular değişmeden çalışır:
//!
//! `varint(version) varint(node_count) varint(edge_count) varint(diller) { varint(uzunluk) bayt… }*`
//!
//! varint: x < 251 tek bayt; < 2^16 için `251` + u16 LE; < 2^32 için `252` + u32 LE;
//! aksi `253` + u64 LE. Alan sırası biçimin parçasıdır; değişiklikte `version` artar.

use crate::WiringMeta;

fn varint(o: &mut Vec<u8>, x: u64) {
    if x < 251 {
        o.push(x as u8);
    } else if x <= u64::from(u16::MAX) {
        o.push(251);
        o.extend_from_slice(&(x as u16).to_le_bytes());
    } else if x <= u64::from(u32::MAX) {
        o.push(252);
        o.extend_from_slice(&(x as u32).to_le_bytes());
    } else {
        o.push(253);
        o.extend_from_slice(&x.to_le_bytes());
    }
}

/// `b`'nin başından bir varint okur: (değer, kalan).
fn oku_varint(b: &[u8]) -> Option<(u64, &[u8])> {
    let (&ilk, r) = b.split_first()?;
    let n = match ilk {
        0..=250 => return Some((u64::from(ilk), r)),
        251 => 2,
        252 => 4,
        253 => 8,
        _ => return None,
    };
    let (sayi, kalan) = r.split_at_checked(n)?;
    let mut tam = [0u8; 8];
    tam[..n].copy_from_slice(sayi);
    Some((u64::from_le_bytes(tam), kalan))
}

impl WiringMeta {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::new();
        varint(&mut o, u64::from(self.version));
        varint(&mut o, self.node_count);
        varint(&mut o, self.edge_count);
        varint(&mut o, self.languages.len() as u64);
        for dil in &self.languages {
            varint(&mut o, dil.len() as u64);
            o.extend_from_slice(dil.as_bytes());
        }
        o
    }

    /// Tam bir `wiring.bin` çözer; fazla bayt, kesik veri ya da geçersiz UTF-8 `None`.
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        let (version, b) = oku_varint(b)?;
        let (node_count, b) = oku_varint(b)?;
        let (edge_count, b) = oku_varint(b)?;
        let (adet, mut b) = oku_varint(b)?;
        let mut languages = Vec::new();
        for _ in 0..adet {
            let (uzunluk, r) = oku_varint(b)?;
            let (dil, r) = r.split_at_checked(usize::try_from(uzunluk).ok()?)?;
            languages.push(String::from_utf8(dil.to_vec()).ok()?);
            b = r;
        }
        b.is_empty().then_some(Self {
            version: u32::try_from(version).ok()?,
            node_count,
            edge_count,
            languages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(n: u64, e: u64) -> WiringMeta {
        WiringMeta {
            version: 1,
            node_count: n,
            edge_count: e,
            languages: vec!["rust".into(), "typescript".into()],
        }
    }

    #[test]
    fn sabit_baytlar() {
        // pasli-beyin `kopru.rs` testindeki sabit baytlarla AYNI.
        let m = WiringMeta {
            version: 1,
            node_count: 2,
            edge_count: 3,
            languages: vec!["rust".to_string()],
        };
        assert_eq!(m.to_bytes(), [1, 2, 3, 1, 4, b'r', b'u', b's', b't']);
    }

    #[test]
    fn varint_sinirlari_gidis_donus() {
        for x in [
            0,
            250,
            251,
            65_535,
            65_536,
            u64::from(u32::MAX),
            u64::from(u32::MAX) + 1,
            u64::MAX,
        ] {
            let m = meta(x, x.saturating_sub(1));
            assert_eq!(WiringMeta::from_bytes(&m.to_bytes()), Some(m), "{x}");
        }
        let mut o = Vec::new();
        for (x, beklenen) in [(250u64, 1), (251, 3), (65_536, 5), (1 << 32, 9)] {
            o.clear();
            varint(&mut o, x);
            assert_eq!(o.len(), beklenen, "{x}");
        }
    }

    #[test]
    fn kesik_fazla_ve_gecersiz_reddedilir() {
        let b = meta(1000, 70_000).to_bytes();
        for n in 0..b.len() {
            assert_eq!(WiringMeta::from_bytes(&b[..n]), None, "kesik {n}");
        }
        let mut fazla = b.clone();
        fazla.push(0);
        assert_eq!(WiringMeta::from_bytes(&fazla), None);
        assert_eq!(
            WiringMeta::from_bytes(&[254]),
            None,
            "bilinmeyen işaret baytı"
        );
        assert_eq!(
            WiringMeta::from_bytes(&[1, 0, 0, 1, 1, 0xFF]),
            None,
            "geçersiz UTF-8"
        );
    }
}
