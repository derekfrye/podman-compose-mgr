use podman_compose_mgr::read_val::{read_val_from_cmd_line_and_proceed, GrammarFragment, GrammarType};

#[test]
fn test1() {
    let mut grammars = vec![
        GrammarFragment {
            original_val_for_prompt: Some("original_val_for_prompt1".to_string()),
            shortened_val_for_prompt: Some("shortened_val_for_prompt1".to_string()),
            pos: 1,
            prefix: Some("prefix1".to_string()),
            suffix: Some("suffix1".to_string()),
            grammar_type: GrammarType::Verbiage,
            display_at_all: true,
            can_shorten: true,
        },
        GrammarFragment {
            original_val_for_prompt: Some("original_val_for_prompt2".to_string()),
            shortened_val_for_prompt: Some("shortened_val_for_prompt2".to_string()),
            pos: 2,
            prefix: Some("prefix2".to_string()),
            suffix: Some("suffix2".to_string()),
            grammar_type: GrammarType::Verbiage,
            display_at_all: true,
            can_shorten: true,
        },
    ];
    let mut grammars = &mut grammars;
    let result = read_val_from_cmd_line_and_proceed(grammars);
    assert_eq!(result.user_entered_val, None);
}