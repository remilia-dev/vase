// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::util::{
    parse_int,
    parse_real,
    NumBase,
    ParseNumberError,
};

// The floating point comparisons are intentional.
#[allow(clippy::float_cmp)]
#[test]
fn number_parsing_matches_standard() -> Result<(), ParseNumberError> {
    for i in 0..100_000usize {
        let mut number = i.to_string();
        let parsed_int = parse_int::<usize>(NumBase::Decimal, number.as_str())?;
        assert_eq!(i, parsed_int.number);
        for j in 0..=number.len() {
            number.insert(j, '.');
            let util_parsed = parse_real::<f32>(NumBase::Decimal, number.as_str())?;
            let std_parsed = number.as_str().parse::<f32>().unwrap();
            assert_eq!(
                util_parsed.number, std_parsed,
                "Difference occured with: {}",
                number
            );
            number.remove(j);
        }
    }
    Ok(())
}
