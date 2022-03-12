// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use super::{
    IntervalKind, NextCardStates, PreviewState, ReschedulingFilterState, ReviewState, StateContext,
};
use crate::revlog::RevlogReviewKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilteredState {
    Preview(PreviewState),
    Rescheduling(ReschedulingFilterState),
}

impl FilteredState {
    pub(crate) fn interval_kind(self) -> IntervalKind {
        match self {
            FilteredState::Preview(state) => state.interval_kind(),
            FilteredState::Rescheduling(state) => state.interval_kind(),
        }
    }

    pub(crate) fn revlog_kind(self) -> Option<RevlogReviewKind> {
        match self {
            FilteredState::Preview(_state) => None,
            FilteredState::Rescheduling(state) => Some(state.revlog_kind()),
        }
    }

    pub(crate) fn next_states(self, ctx: &StateContext) -> NextCardStates {
        match self {
            FilteredState::Preview(state) => state.next_states(ctx),
            FilteredState::Rescheduling(state) => state.next_states(ctx),
        }
    }

    pub(crate) fn review_state(self) -> Option<ReviewState> {
        match self {
            FilteredState::Preview(_) => None,
            FilteredState::Rescheduling(state) => state.original_state.review_state(),
        }
    }
}
