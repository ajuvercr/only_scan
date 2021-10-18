use super::*;

#[test]
fn test_parsing() {
    if let Some(item) = parse_text("test 13.20\n") {
        assert_eq!(item.name, "test");
        assert_eq!(item.price, 13.2);
    } else {
        assert!(false);
    }
}

#[test]
fn test_alpro() {
    if let Some(item) = parse_text("1L ALP DRINK AMAND 2 29 \n") {
        assert_eq!(item.name, "1L ALP DRINK AMAND");
        assert_eq!(item.price, 2.29);
    } else {
        assert!(false);
    }
}

#[test]
fn test_vuilzak_groen() {
    if let Some(item) = parse_text("VUILZAK GROEN 30L - 1110 \n") {
        assert_eq!(item.name, "VUILZAK GROEN 30L -");
        assert_eq!(item.price, 11.10);
    } else {
        assert!(false);
    }
}

#[test]
fn test_fail_1() {
    assert_eq!(parse_text("  \n"), None);
}

#[test]
fn test_fail_2() {
    assert_eq!(parse_text("50CL. MNSTR PARADIS 1f42 "), None);
}
