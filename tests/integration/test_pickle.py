def test_pickle_support():
    import pickle
    import interegular
    from outlines_core.fsm.outlines_core_rs import Index, Vocabulary
    
    from outlines_core.fsm.regex import (
        create_fsm_index_tokenizer,
        make_byte_level_fsm,
        make_deterministic_fsm,
        reduced_vocabulary,
    )

    class MockTokenizer:
        vocabulary = {"a": 1, "b": 2, "z": 3, "eos": 4}
        special_tokens = {"eos"}
        eos_token_id = 4

        def convert_token_to_string(self, token):
            return token

    tokenizer = MockTokenizer()

    pattern = r"z[ab]z"
    regex_pattern = interegular.parse_pattern(pattern)
    interegular_fsm = regex_pattern.to_fsm().reduce()
    regex_fsm, _ = make_deterministic_fsm(interegular_fsm)
    tokens_to_token_ids, _ = reduced_vocabulary(tokenizer)

    vocabulary = Vocabulary.from_dict(tokens_to_token_ids)
    fsm_info = regex_fsm.fsm_info
    
    index = Index(fsm_info, vocabulary, 4, frozenset())

    pickled = pickle.dumps(index)
    restored = pickle.loads(pickled)

  # assert index.initial == restored.initial
  # assert index.finals == restored.finals
  # assert index.states_to_token_subsets == restored.states_to_token_subsets
  # assert index.eos_token_id == restored.eos_token_id