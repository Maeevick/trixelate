//! Reading back the PDFs the crate writes, for its tests: its objects and their dictionaries.

use std::collections::BTreeMap;

/// Every object of a PDF by its number, from where its cross-reference table says it starts, which must be there.
pub(crate) fn objects_of(pdf: &[u8]) -> BTreeMap<u32, &[u8]> {
    let text = String::from_utf8_lossy(pdf);
    let table_at: usize = text
        .rsplit_once("startxref\n")
        .and_then(|(_, rest)| rest.lines().next())
        .and_then(|line| line.parse().ok())
        .unwrap_or_default();
    let table = String::from_utf8_lossy(&pdf[table_at..]);
    let mut lines = table.lines();
    assert_eq!(lines.next(), Some("xref"));
    let count: u32 = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|count| count.parse().ok())
        .unwrap_or_default();
    lines
        .skip(1)
        .take(count as usize - 1)
        .zip(1..)
        .map(|(entry, number)| {
            let at: usize = entry[..10].parse().unwrap_or_default();
            let object = &pdf[at..];
            assert!(object.starts_with(format!("{number} 0 obj\n").as_bytes()));
            (number, object)
        })
        .collect()
}

pub(crate) fn dictionary_of(objects: &BTreeMap<u32, &[u8]>, number: u32) -> String {
    let object = String::from_utf8_lossy(objects.get(&number).copied().unwrap_or_default());
    let end = [object.find("stream\n"), object.find("endobj")]
        .into_iter()
        .flatten()
        .min();
    object[..end.unwrap_or_default()].to_string()
}

pub(crate) fn number_after(text: &str, key: &str) -> u32 {
    text.split_once(key)
        .and_then(|(_, rest)| rest.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|number| number.parse().ok())
        .unwrap_or_default()
}

pub(crate) fn numbers_referred_to(text: &str, key: &str) -> Vec<u32> {
    text.split_once(key)
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(references, _)| {
            references
                .split(" 0 R")
                .filter_map(|number| number.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}
