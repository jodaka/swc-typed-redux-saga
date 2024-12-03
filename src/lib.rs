use std::collections::HashSet;
use swc_core::{
    ecma::ast::{Callee, Expr, ImportDecl, ImportSpecifier, Program, YieldExpr},
    ecma::visit::{Fold, FoldWith, VisitMut, VisitMutWith},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

struct TransformVisitor {
    local_idents: HashSet<String>,
}

impl TransformVisitor {
    pub fn new() -> Self {
        Self {
            local_idents: HashSet::new(),
        }
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        if &import_decl.src.value != "typed-redux-saga/macro" {
            return;
        }

        for specifier in &import_decl.specifiers {
            if let ImportSpecifier::Named(local) = specifier {
                self.local_idents.insert(local.local.sym.to_string());
            }
        }

        import_decl.src.raw = None;
        import_decl.src.value = "redux-saga/effects".into();
    }

    fn visit_mut_yield_expr(&mut self, yield_expr: &mut YieldExpr) {
        if let Some(arg) = &yield_expr.arg {
            if let Expr::Call(call_expr) = &**arg {
                if let Callee::Expr(callee_expr) = &call_expr.callee {
                    if let Expr::Ident(id) = &**callee_expr {
                        if self.local_idents.contains(&id.sym.to_string()) {
                            yield_expr.delegate = false;
                        }
                    }
                }
            }
        }

        yield_expr.visit_mut_children_with(self);
    }
}

impl Fold for TransformVisitor {
    fn fold_program(&mut self, program: Program) -> Program {
        program.fold_children_with(self)
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut TransformVisitor::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::common::FileName;
    use swc_core::common::SourceMap;
    use swc_core::ecma::codegen::{text_writer::JsWriter, Emitter};
    use swc_core::ecma::parser::{Parser, StringInput};
    use swc_core::ecma::transforms::testing::{test, test_fixture};
    use swc_core::ecma::visit::Fold;

    fn transform_visitor() -> impl Fold {
        TransformVisitor::new()
    }

    // Test for import transformation
    test!(
        Syntax::Es(Default::default()),
        |_| transform_visitor(),
        replaces_import,
        r#"import { put } from "typed-redux-saga/macro";"#,
        r#"import { put } from "redux-saga/effects";"#
    );

    // Test for yield expression transformation
    test!(
        Syntax::Es(Default::default()),
        |_| transform_visitor(),
        replaces_yield,
        r#"
        import { put } from "typed-redux-saga/macro";
        function* test() { yield* put(); }
        "#,
        r#"
        import { put } from "redux-saga/effects";
        function* test() { yield put(); }
        "#
    );
}
