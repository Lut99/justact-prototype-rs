//  DRIVER.rs
//    by Lut99
//
//  Created:
//    27 Jan 2025, 10:52:52
//  Last edited:
//    27 Jan 2025, 10:53:40
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a driver agent, which is programmed with a workflow to
//!   execute on the `worker`.
//


/***** AUXILLARY *****/
/// The definition of tasks in a [`Workflow`].





/***** LIBRARY *****/
pub struct Driver {
    /// The workflow to execute.
    workflow: Workflow,
    /// A counter for allocating message identifiers.
    next_id:  usize,
}
