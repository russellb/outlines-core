// use std::collections::{BTreeMap, BTreeSet};

// use ::outlines_core as core_lib;
// use pyo3::types::{PyAnyMethods, PyDict, PyModule, PyModuleMethods, PySet};
use pyo3::{pyfunction, pymodule, wrap_pyfunction, Bound, PyResult};

#[pymodule]
mod _lib {
    use outlines_core::FLAG;
    use pyo3::prelude::*;

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }

    #[pyfunction]
    fn show_me_the_flag() -> PyResult<String> {
        Ok(FLAG.to_string())
    }

    #[pyfunction]
    fn anotherone() -> PyResult<String> {
        Ok("This is another one".to_string())
    }
}

// #[pymodule]
// fn outlines_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(add, m)?)?;
//     Ok(())
// }

// #[pyfunction]
// Create an FSM index end-to-end.
//
// Args:
//     fsm_transitions (Dict[Tuple[int, int], int]): FSM transitions mapping.
//     alphabet_symbol_mapping (Dict[str, int]): Alphabet symbol mapping.
//     alphabet_anything_value (int): Value representing 'anything' in the alphabet.
//     fsm_initial (int): Initial state of the FSM.
//     fsm_finals (Set[int]): Set of final states in the FSM.
//     vocabulary (Dict[str, List[int]]): Vocabulary mapping.
//
// Returns:
//     Dict[int, Set[Tuple[int, int]]]: The created FSM index.
//
// Raises:
//     ValueError: If the input types are incorrect or conversion fails.
// fn create_fsm_index_end_to_end(
//     fsm_transitions: Bound<PyDict>,
//     alphabet_symbol_mapping: Bound<PyDict>,
//     alphabet_anything_value: i32,
//     fsm_initial: i32,
//     fsm_finals: Bound<PySet>,
//     vocabulary: Bound<PyDict>,
// ) -> PyResult<BTreeMap<i32, BTreeSet<(i32, i32)>>> {
//     let fsm_transitions_map = fsm_transitions.extract::<BTreeMap<(i32, i32), i32>>()?;
//     let alphabet_symbol_mapping_map = alphabet_symbol_mapping.extract::<BTreeMap<char, i32>>()?;
//     let fsm_finals_set = fsm_finals.extract::<BTreeSet<i32>>()?;
//     let vocabulary_map = vocabulary.extract::<BTreeMap<String, Vec<i32>>>()?;

//     let res = core_lib::create_fsm_index_end_to_end_rust(
//         &fsm_transitions_map,
//         &alphabet_symbol_mapping_map,
//         alphabet_anything_value,
//         fsm_initial,
//         &fsm_finals_set,
//         &vocabulary_map,
//     );

//     Ok(res)
// }

// Outlines is a Generative Model Programming Framework.
// #[pymodule]
// fn outlines_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(create_fsm_index_end_to_end, m)?)?;
//     Ok(())
// }
