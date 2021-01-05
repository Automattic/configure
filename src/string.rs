pub fn distance_between_strings_in(
    string1: &str,
    string2: &str,
    strings: &Vec<String>,
) -> Option<i32> {
    let str1_ix = index_of_string_in(string1, &strings);
    let str2_ix = index_of_string_in(string2, &strings);

    if str1_ix.is_some() && str2_ix.is_some() {
        return Some((str1_ix.unwrap() - str2_ix.unwrap()).abs());
    } else {
        return None;
    }
}

fn index_of_string_in(string: &str, strings: &Vec<String>) -> Option<i32> {
    match strings.iter().position(|r| r == string) {
        Some(ix) => Some(ix as i32),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::string::distance_between_strings_in;
    use crate::string::index_of_string_in;

    #[test]
    fn test_that_index_of_string_in_works() {
        assert!(index_of_string_in("one", &test_vec()) == Some(0))
    }

    #[test]
    fn test_that_index_of_string_returns_none_if_value_not_present() {
        assert!(index_of_string_in("foo", &test_vec()) == None)
    }

    #[test]
    fn test_that_distance_between_strings_is_zero_for_identical_strings() {
        assert!(distance_between_strings_in("one", "one", &test_vec()) == Some(0))
    }

    #[test]
    fn test_that_distance_between_strings_is_one_for_sequential_strings() {
        assert!(distance_between_strings_in("one", "two", &test_vec()) == Some(1))
    }

    #[test]
    fn test_that_distance_between_strings_is_correct_for_strings() {
        assert!(distance_between_strings_in("one", "three", &test_vec()) == Some(2))
    }

    #[test]
    fn test_that_distance_between_strings_is_none_for_one_missing_string() {
        assert!(distance_between_strings_in("one", "foo", &test_vec()) == None)
    }

    #[test]
    fn test_that_distance_between_strings_is_none_for_both_missing_strings() {
        assert!(distance_between_strings_in("foo", "bar", &test_vec()) == None)
    }

    // Test Helpers
    fn test_vec() -> Vec<String> {
        vec!["one".to_string(), "two".to_string(), "three".to_string()]
    }
}
