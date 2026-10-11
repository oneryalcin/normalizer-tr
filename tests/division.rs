//! Compact division read as written, with component source coordinates.
use normalizer_tr::{AmbiguityPolicy, NormalizeOptions, Normalizer, SegmentKind, SourceRange};

fn options(policy: AmbiguityPolicy) -> NormalizeOptions {
    NormalizeOptions {
        ambiguity_policy: policy,
        ..Default::default()
    }
}

#[test]
fn compact_division_is_read_as_written_in_every_policy() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        ("3/4", "üç bölü dört"),
        ("3/4 pizza", "üç bölü dört pizza"),
        ("pizzanın 3/4'ü", "pizzanın üç bölü dördü"),
        ("3/4'ten", "üç bölü dörtten"),
        ("(1/2)", "(bir bölü iki)"),
        (
            "3/4, 1/2 ve 2/3.",
            "üç bölü dört, bir bölü iki ve iki bölü üç.",
        ),
        ("24/7", "yirmi dört bölü yedi"),
        ("0/5", "sıfır bölü beş"),
        ("3/0", "üç bölü sıfır"),
    ] {
        for policy in [
            AmbiguityPolicy::Preserve,
            AmbiguityPolicy::Reject,
            AmbiguityPolicy::Fallback,
        ] {
            let r = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(r.normalized_text(), expected, "{source} {policy:?}");
            assert!(r.complete() && r.issues().is_empty() && r.fallbacks().is_empty());
            let mut cursor = 0;
            for segment in r.segments() {
                assert_eq!(segment.range().start(), cursor);
                cursor = segment.range().end();
                assert!(source.is_char_boundary(cursor));
            }
            assert_eq!(cursor, source.len());
        }
    }
}

#[test]
fn division_components_partition_the_original_source() {
    let source = "ö 12/4'ü";
    let r = Normalizer::new()
        .unwrap()
        .normalize(source, &NormalizeOptions::default())
        .unwrap();
    let records: Vec<_> = r
        .segments()
        .iter()
        .map(|s| (s.range(), s.kind(), s.text(), s.rule_id()))
        .collect();
    assert_eq!(
        records[1..],
        [
            (
                SourceRange::new(3, 5),
                SegmentKind::Cardinal,
                "on iki",
                "number"
            ),
            (
                SourceRange::new(5, 6),
                SegmentKind::Symbol,
                " bölü ",
                "expression.division"
            ),
            (
                SourceRange::new(6, 10),
                SegmentKind::Cardinal,
                "dördü",
                "number"
            ),
        ]
    );
}

#[test]
fn other_slash_notation_is_not_division() {
    let n = Normalizer::new().unwrap();
    for source in [
        "03/04",
        "1/2/3",
        "3/4/",
        "-3/4",
        "3/-4",
        "1,5/2",
        "1.000/3",
        "3 / 4",
        "3/4 kg",
        "3/x",
        "x/4",
        "3/4x",
        "2/3'si",
        "3/4'üncü",
        "14/03/2026",
        "1/1000000000000000",
    ] {
        let r = n
            .normalize(source, &options(AmbiguityPolicy::Preserve))
            .unwrap();
        assert!(!r.complete(), "{source}");
        assert_eq!(r.normalized_text(), source);
        let fallback = n
            .normalize(source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert!(!fallback.normalized_text().contains("bölü"), "{source}");
    }
}

#[test]
fn dates_addresses_and_rates_keep_their_slash_readings() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        ("tarih 03/04/2026", "tarih üç Nisan iki bin yirmi altı"),
        ("2026/12/31", "otuz bir Aralık iki bin yirmi altı"),
        (
            "adres: No:12/4",
            "adres: bina numarası on iki, daire numarası dört",
        ),
        ("5 km/sa", "beş kilometre saat"),
    ] {
        let r = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(r.normalized_text(), expected, "{source}");
    }
}
