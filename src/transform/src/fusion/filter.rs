// Copyright Materialize, Inc. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Fuses multiple `Filter` operators into one; deduplicates predicates.
//!
//! ```rust
//! use expr::{MirRelationExpr, MirScalarExpr};
//! use repr::{ColumnType, Datum, RelationType, ScalarType};
//!
//! use transform::fusion::filter::Filter;
//!
//! let input = MirRelationExpr::constant(vec![], RelationType::new(vec![
//!     ScalarType::Bool.nullable(false),
//! ]));
//!
//! let predicate0 = MirScalarExpr::Column(0);
//! let predicate1 = MirScalarExpr::Column(0);
//! let predicate2 = MirScalarExpr::Column(0);
//!
//! let mut expr =
//! input
//!     .clone()
//!     .filter(vec![predicate0.clone()])
//!     .filter(vec![predicate1.clone()])
//!     .filter(vec![predicate2.clone()]);
//!
//! // .transform() will deduplicate any predicates
//! use transform::{Transform, TransformArgs};
//! Filter.transform(&mut expr, TransformArgs {
//!   id_gen: &mut Default::default(),
//!   indexes: &std::collections::HashMap::new(),
//! });
//!
//! let correct = input.filter(vec![predicate0]);
//!
//! assert_eq!(expr, correct);
//! ```

use crate::TransformArgs;
use expr::{MirRelationExpr, MirScalarExpr};

/// Fuses multiple `Filter` operators into one and deduplicates predicates.
#[derive(Debug)]
pub struct Filter;

impl crate::Transform for Filter {
    fn transform(
        &self,
        relation: &mut MirRelationExpr,
        _: TransformArgs,
    ) -> Result<(), crate::TransformError> {
        relation.visit_mut_pre(&mut |e| {
            self.action(e);
        });
        Ok(())
    }
}

impl Filter {
    /// Fuses multiple `Filter` operators into one and deduplicates predicates.
    pub fn action(&self, relation: &mut MirRelationExpr) {
        if let MirRelationExpr::Filter { input, predicates } = relation {
            // consolidate nested filters.
            while let MirRelationExpr::Filter {
                input: inner,
                predicates: p2,
            } = &mut **input
            {
                predicates.extend(p2.drain(..));
                *input = Box::new(inner.take_dangerous());
            }

            for predicate in predicates.iter_mut() {
                canonicalize_predicate(predicate);
            }
            predicates.sort();
            predicates.dedup();

            // remove the Filter stage if empty.
            if predicates.is_empty() {
                *relation = input.take_dangerous();
            }
        }
    }
}

/// Ensures that two equalities are made in a consistent order.
fn canonicalize_predicate(predicate: &mut MirScalarExpr) {
    if let MirScalarExpr::CallBinary {
        func: expr::BinaryFunc::Eq,
        expr1,
        expr2,
    } = predicate
    {
        // Canonically order elements so that deduplication works better.
        if expr2 < expr1 {
            ::std::mem::swap(expr1, expr2);
        }

        // Comparison to self is always true unless the element is `Datum::Null`.
        if expr1 == expr2 {
            *predicate = expr1
                .clone()
                .call_unary(expr::UnaryFunc::IsNull)
                .call_unary(expr::UnaryFunc::Not);
        }
    }
}
