use std::borrow::Cow;
use rustc::hir::*;
use rustc::hir::intravisit::{walk_expr, NestedVisitorMap, Visitor};
use rustc::lint::LateContext;
use syntax::ast::Name;
use syntax::codemap::Span;
use utils::{get_pat_name, match_var, snippet};

pub fn get_spans(
    cx: &LateContext,
    opt_body_id: Option<BodyId>,
    idx: usize,
    replacements: &'static [(&'static str, &'static str)],
) -> Option<Vec<(Span, Cow<'static, str>)>> {
    if let Some(body) = opt_body_id.map(|id| cx.tcx.hir.body(id)) {
        get_binding_name(&body.arguments[idx])
            .map_or_else(|| Some(vec![]), |name| extract_clone_suggestions(cx, name, replacements, body))
    } else {
        Some(vec![])
    }
}

fn extract_clone_suggestions<'a, 'tcx: 'a>(
    cx: &LateContext<'a, 'tcx>,
    name: Name,
    replace: &'static [(&'static str, &'static str)],
    body: &'tcx Body,
) -> Option<Vec<(Span, Cow<'static, str>)>> {
    let mut visitor = PtrCloneVisitor {
        cx,
        name,
        replace,
        spans: vec![],
        abort: false,
    };
    visitor.visit_body(body);
    if visitor.abort {
        None
    } else {
        Some(visitor.spans)
    }
}

struct PtrCloneVisitor<'a, 'tcx: 'a> {
    cx: &'a LateContext<'a, 'tcx>,
    name: Name,
    replace: &'static [(&'static str, &'static str)],
    spans: Vec<(Span, Cow<'static, str>)>,
    abort: bool,
}

impl<'a, 'tcx: 'a> Visitor<'tcx> for PtrCloneVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr) {
        if self.abort {
            return;
        }
        if let ExprMethodCall(ref seg, _, ref args) = expr.node {
            if args.len() == 1 && match_var(&args[0], self.name) {
                if seg.name == "capacity" {
                    self.abort = true;
                    return;
                }
                for &(fn_name, suffix) in self.replace {
                    if seg.name == fn_name {
                        self.spans
                            .push((expr.span, snippet(self.cx, args[0].span, "_") + suffix));
                        return;
                    }
                }
            }
            return;
        }
        walk_expr(self, expr);
    }

    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }
}

fn get_binding_name(arg: &Arg) -> Option<Name> {
    get_pat_name(&arg.pat)
}
