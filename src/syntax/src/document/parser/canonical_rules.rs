// Generated from docs/design/grammar-audit/productions.tsv.
// Do not edit by hand.

use crate::document::RuleId;

pub const CANONICAL_RULE_COUNT: usize = 540;

pub static CANONICAL_RULES: &[(&str, RuleId)] = &[
  ("absolute-source-import-specifier", RuleId(0x37410b35)),
  ("abstract-el", RuleId(0xed0e65f3)),
  ("abstract-sigil", RuleId(0xae0225c6)),
  ("activation-arm", RuleId(0xc5b32bea)),
  ("activation-scope", RuleId(0x453304d4)),
  ("add", RuleId(0x3b391274)),
  ("add-assign-operator", RuleId(0x77538e15)),
  ("add-sub-operator", RuleId(0x2b923e96)),
  ("aliased-item-import", RuleId(0xb7bcdda4)),
  ("alignment-separator", RuleId(0xaea70f10)),
  ("alpha", RuleId(0x5d8b6dab)),
  ("alpha-token", RuleId(0xab243f87)),
  ("alphanumeric", RuleId(0x0b09b0d6)),
  ("ampersand", RuleId(0xebb16704)),
  ("and", RuleId(0x0f29c2a6)),
  ("any", RuleId(0x2c29f04d)),
  ("any-token", RuleId(0xd91e0675)),
  ("apostrophe", RuleId(0xdff65064)),
  ("argument-list", RuleId(0xc1b3b46d)),
  ("assign-operator", RuleId(0x688f2a93)),
  ("asterisk", RuleId(0xef8a6081)),
  ("async-transition-operator", RuleId(0xa37fbb00)),
  ("at", RuleId(0x57251588)),
  ("atom", RuleId(0x037448d8)),
  ("backslash", RuleId(0x6f17fb77)),
  ("bar", RuleId(0x76b77d1a)),
  ("bare-source-import-specifier", RuleId(0x49acf5a0)),
  ("binary-literal", RuleId(0x521c71d2)),
  ("binding", RuleId(0xc5955a62)),
  ("blank-line", RuleId(0x8abba276)),
  ("body", RuleId(0xdbaa7975)),
  ("boolean", RuleId(0x65f46ebf)),
  ("box-bl", RuleId(0xc5e59dc3)),
  ("box-bl-bold", RuleId(0x4d5fd5cd)),
  ("box-bl-round", RuleId(0xba2cab7a)),
  ("box-br", RuleId(0xc3e59a9d)),
  ("box-br-bold", RuleId(0xc9640063)),
  ("box-br-round", RuleId(0xb4d94328)),
  ("box-cross", RuleId(0xb51d1803)),
  ("box-drawing-char", RuleId(0x8ee2110c)),
  ("box-drawing-emoji", RuleId(0xb0ab856e)),
  ("box-horz", RuleId(0x3bbd73d4)),
  ("box-t-bottom", RuleId(0x5e60b5b1)),
  ("box-t-left", RuleId(0x33aad62f)),
  ("box-t-right", RuleId(0xc046cc20)),
  ("box-t-top", RuleId(0x359df285)),
  ("box-tl", RuleId(0xc9ff3bf5)),
  ("box-tl-bold", RuleId(0xc7baef3b)),
  ("box-tl-round", RuleId(0x73142f90)),
  ("box-tr", RuleId(0xbbff25eb)),
  ("box-tr-bold", RuleId(0x58c0be35)),
  ("box-tr-round", RuleId(0x1f0a5952)),
  ("box-vert", RuleId(0x7757ebaa)),
  ("box-vert-bold", RuleId(0x74980a36)),
  ("brace-subscript", RuleId(0x263793c4)),
  ("bracket-subscript", RuleId(0xfcb4c29b)),
  ("call-arg", RuleId(0x77ead188)),
  ("call-arg-with-binding", RuleId(0x8be85223)),
  ("caret", RuleId(0x7aef16a8)),
  ("carriage-return", RuleId(0xdde3fe8c)),
  ("carriage-return-new-line", RuleId(0xff2066c8)),
  ("cd-rpl", RuleId(0x8c2f9121)),
  ("center-alignment", RuleId(0xb4b26da4)),
  ("check-list", RuleId(0x799fadea)),
  ("check-list-item", RuleId(0x0846ce66)),
  ("check-mark", RuleId(0xc1f3e14f)),
  ("checked-item", RuleId(0x20377dea)),
  ("citation", RuleId(0xd92e6394)),
  ("clc-rpl", RuleId(0x010d7744)),
  ("clear-rpl", RuleId(0xae623141)),
  ("code-block", RuleId(0x47edebd4)),
  ("code-rpl", RuleId(0xb1801777)),
  ("code-terminal", RuleId(0x7019f177)),
  ("codeblock-sigil", RuleId(0x7b2e0578)),
  ("colon", RuleId(0x497e753c)),
  ("comma", RuleId(0xbffa8578)),
  ("comment", RuleId(0x67a6c45e)),
  ("comment-sigil", RuleId(0xbf72dce3)),
  ("comparison-operator", RuleId(0xb678194b)),
  ("complement", RuleId(0x5282471d)),
  ("complex-number", RuleId(0x6a69227b)),
  ("comprehension-qualifier", RuleId(0xe50e092e)),
  ("context-address-path", RuleId(0x94fb8e83)),
  ("context-address-path-token", RuleId(0x19a621df)),
  ("context-base-context", RuleId(0x33efe980)),
  ("context-base-resource-uri", RuleId(0xfa7d8ac6)),
  ("context-capability-declaration", RuleId(0x122a0454)),
  ("context-capability-path", RuleId(0x9d4416c7)),
  ("context-capability-path-token", RuleId(0xc65e4f0b)),
  ("context-capability-scope", RuleId(0x7fb1bcc4)),
  ("context-declaration", RuleId(0xc5776b35)),
  ("context-import-alias-segment", RuleId(0xdf58baa7)),
  ("context-send", RuleId(0xdab0da55)),
  ("cross", RuleId(0x29f5189b)),
  ("cross-product", RuleId(0xbca81ce1)),
  ("dash", RuleId(0x0179def5)),
  ("decimal-literal", RuleId(0x04e47722)),
  ("define-operator", RuleId(0xdeffa145)),
  ("difference", RuleId(0xea8f6e42)),
  ("digit", RuleId(0x885c8a56)),
  ("digit-sequence", RuleId(0x70208d0c)),
  ("digit-token", RuleId(0x416fb636)),
  ("div-assign-operator", RuleId(0x2121da89)),
  ("divide", RuleId(0x61526270)),
  ("docs-rpl", RuleId(0x3f0b9bf1)),
  ("dollar", RuleId(0xf364bd3f)),
  ("dot-product", RuleId(0x6bf9d186)),
  ("dot-subscript", RuleId(0xad92070c)),
  ("dot-subscript-int", RuleId(0x40db609c)),
  ("element-of", RuleId(0x8284dd6f)),
  ("emoji", RuleId(0x4a90ef3d)),
  ("emoji-grapheme", RuleId(0xce4c615d)),
  ("emphasis", RuleId(0x5ff3a9f7)),
  ("emphasis-sigil", RuleId(0xfdce2882)),
  ("empty", RuleId(0x18a7beee)),
  ("empty-map", RuleId(0xe8a9c9f5)),
  ("empty-paragraph", RuleId(0xe7a8e3ad)),
  ("empty-set", RuleId(0xa0d9b56f)),
  ("english-false-literal", RuleId(0x04f87d33)),
  ("english-true-literal", RuleId(0xbcec4d60)),
  ("enum-define", RuleId(0xa8bdcda4)),
  ("enum-separator", RuleId(0x57266112)),
  ("enum-variant", RuleId(0xfec00678)),
  ("enum-variant-inline-kind", RuleId(0xea7ed22f)),
  ("enum-variant-kind", RuleId(0x2b5cf505)),
  ("equal", RuleId(0x2f7508ef)),
  ("equal-to", RuleId(0x00e4debb)),
  ("equation", RuleId(0x943adfd3)),
  ("equation-sigil", RuleId(0x61878026)),
  ("error-alt-sigil", RuleId(0x85d7a022)),
  ("error-block", RuleId(0x44314f6d)),
  ("error-sigil", RuleId(0x41a3f3f4)),
  ("escaped-char", RuleId(0x7df2caa9)),
  ("eval-inline-mech-code", RuleId(0x5a5b75d7)),
  ("exclamation", RuleId(0xc234a5d6)),
  ("exp-assign-operator", RuleId(0xa17bd969)),
  ("export-declaration", RuleId(0x34842f0a)),
  ("expression", RuleId(0xcf15afeb)),
  ("factor", RuleId(0x5c8ff3a6)),
  ("false-literal", RuleId(0x03c29ce6)),
  ("fancy-table", RuleId(0xdaf03453)),
  ("fancy-table-header", RuleId(0xa4e5b5a3)),
  ("field", RuleId(0x67826267)),
  ("figure-item", RuleId(0xd09598c3)),
  ("figures", RuleId(0x33a20f0e)),
  ("figures-row", RuleId(0xbf987a37)),
  ("float", RuleId(0xa6c45d85)),
  ("float-decimal-start", RuleId(0xb3b229f6)),
  ("float-full", RuleId(0x2316b35d)),
  ("float-left", RuleId(0xd95b4809)),
  ("float-literal", RuleId(0xc3d7b977)),
  ("float-right", RuleId(0x0d2337aa)),
  ("float-sigil", RuleId(0x8d57ea10)),
  ("footnote", RuleId(0xdda62331)),
  ("footnote-prefix", RuleId(0x12f25b90)),
  ("footnote-reference", RuleId(0x25ddcf69)),
  ("forbidden-emoji", RuleId(0xa330fe15)),
  ("formula", RuleId(0x798fbc5d)),
  ("formula-subscript", RuleId(0x3b3f7c19)),
  ("fsm", RuleId(0xcbd59f49)),
  ("fsm-args", RuleId(0x7aa190c1)),
  ("fsm-arm", RuleId(0x4ee1df36)),
  ("fsm-async-transition", RuleId(0x0bdf7a88)),
  ("fsm-block-transition", RuleId(0x8f392bab)),
  ("fsm-comment-arm", RuleId(0x7aef4356)),
  ("fsm-declare", RuleId(0xd4b2abc2)),
  ("fsm-guard", RuleId(0x6a7c9ca1)),
  ("fsm-guard-arm", RuleId(0x83c43a4e)),
  ("fsm-implementation", RuleId(0x475b3a3e)),
  ("fsm-instance", RuleId(0x9d5b7817)),
  ("fsm-output", RuleId(0x68e572c1)),
  ("fsm-pipe", RuleId(0x8ecde250)),
  ("fsm-specification", RuleId(0x2297b501)),
  ("fsm-state-definition", RuleId(0xb256027d)),
  ("fsm-state-definition-variables", RuleId(0xb2347839)),
  ("fsm-state-transition", RuleId(0x717cb24f)),
  ("fsm-statement-transition", RuleId(0x665eb907)),
  ("fsm-transition", RuleId(0xbba72217)),
  ("fsm-value", RuleId(0xb3b95dc5)),
  ("full-join", RuleId(0x112a81a3)),
  ("function-arg", RuleId(0x8409bba8)),
  ("function-call", RuleId(0xfcbdb56c)),
  ("function-define", RuleId(0xb0b7ceff)),
  ("function-define-match-arms", RuleId(0x92fe8211)),
  ("function-define-statements", RuleId(0x0e0bf554)),
  ("function-match-arm", RuleId(0x15839c42)),
  ("function-out-arg", RuleId(0x9177c6a5)),
  ("function-out-args", RuleId(0xd58e02e2)),
  ("gen-operator", RuleId(0xa54f7858)),
  ("generator", RuleId(0x6eec35c2)),
  ("generator-arrow", RuleId(0x1b9c235e)),
  ("generator-arrow-u", RuleId(0x87d60d34)),
  ("grammar", RuleId(0x4b536ffe)),
  ("grammar-definition", RuleId(0x4f332cec)),
  ("grammar-expression", RuleId(0xca4269bf)),
  ("grammar-factor", RuleId(0x27c3d872)),
  ("grammar-group", RuleId(0xfce872a0)),
  ("grammar-identifier", RuleId(0x759b4f92)),
  ("grammar-list", RuleId(0xfc4ab7f5)),
  ("grammar-not", RuleId(0x5a19eb5e)),
  ("grammar-optional", RuleId(0xec6546b5)),
  ("grammar-peek", RuleId(0x9b771a32)),
  ("grammar-range", RuleId(0xf923b3f6)),
  ("grammar-repeat0", RuleId(0xd3934312)),
  ("grammar-repeat1", RuleId(0xd49344a5)),
  ("grammar-rule", RuleId(0x7cb77e2f)),
  ("grammar-term", RuleId(0x8aa0e583)),
  ("grammar-terminal", RuleId(0x36f7809d)),
  ("grammar-terminal-token", RuleId(0xdc8237a5)),
  ("grave", RuleId(0x9068bb32)),
  ("grave-codeblock-sigil", RuleId(0x7d321eb0)),
  ("greater-than", RuleId(0x57a89e97)),
  ("greater-than-equal", RuleId(0x0ac93612)),
  ("grouping-symbol", RuleId(0x4ec76763)),
  ("guard-operator", RuleId(0x11c1cce1)),
  ("hashtag", RuleId(0x580caca7)),
  ("header-field", RuleId(0x8fed2869)),
  ("help-rpl", RuleId(0xbdc66d79)),
  ("hexadecimal-literal", RuleId(0x38b815ae)),
  ("highlight", RuleId(0x1c9ff127)),
  ("highlight-sigil", RuleId(0x309033d2)),
  ("http-prefix", RuleId(0x185c9154)),
  ("hyperlink", RuleId(0xb189102d)),
  ("idea-block", RuleId(0x88ff8e92)),
  ("idea-sigil", RuleId(0xe8bf28d7)),
  ("identifier", RuleId(0x28a5a83e)),
  ("identifier-path-segment", RuleId(0xa5b1745e)),
  ("identifier-path-segment-emoji", RuleId(0xa36539c1)),
  ("identifier-symbol", RuleId(0xb3aeb85d)),
  ("img", RuleId(0x84e72504)),
  ("img-prefix", RuleId(0xef2025ff)),
  ("import-alias-operator", RuleId(0xddb49ce8)),
  ("import-declaration", RuleId(0x38f7ba9d)),
  ("import-group-item", RuleId(0x583a450e)),
  ("import-group-items", RuleId(0x60bb63c7)),
  ("import-group-separator", RuleId(0xcdfd49c4)),
  ("info-block", RuleId(0x36c95509)),
  ("info-sigil", RuleId(0x227e4790)),
  ("inline-code", RuleId(0xcced3fa2)),
  ("inline-equation", RuleId(0xc6764779)),
  ("inline-mech-code", RuleId(0x833b5950)),
  ("inline-paragraph", RuleId(0xbc63fc47)),
  ("inline-table", RuleId(0x11f04f9d)),
  ("inline-table-header", RuleId(0x162ff185)),
  ("inline-table-row", RuleId(0x4f6fdcb4)),
  ("integer-literal", RuleId(0xff5fa0b7)),
  ("intersection", RuleId(0x6be1e5c8)),
  ("invariant-define", RuleId(0x8a084469)),
  ("join", RuleId(0xc922bc79)),
  ("kind", RuleId(0xd913e243)),
  ("kind-annotation", RuleId(0x879cbcf3)),
  ("kind-any", RuleId(0xb1c044fc)),
  ("kind-atom", RuleId(0x530824e7)),
  ("kind-define", RuleId(0x42029f05)),
  ("kind-empty", RuleId(0x52549b1f)),
  ("kind-kind", RuleId(0x86b3944c)),
  ("kind-map", RuleId(0x03c3eed4)),
  ("kind-matrix", RuleId(0x35c4f5e7)),
  ("kind-record", RuleId(0x4b6afb9f)),
  ("kind-scalar", RuleId(0xed70de7a)),
  ("kind-set", RuleId(0xad43176a)),
  ("kind-table", RuleId(0xbadb583e)),
  ("kind-tuple", RuleId(0x45e1a60c)),
  ("kind-with-option", RuleId(0xd0ad1bd0)),
  ("l1", RuleId(0x18317e4e)),
  ("l2", RuleId(0x17317cbb)),
  ("l3", RuleId(0x16317b28)),
  ("l4", RuleId(0x1d31862d)),
  ("l5", RuleId(0x1c31849a)),
  ("l6", RuleId(0x1b318307)),
  ("l7", RuleId(0x1a318174)),
  ("left-alignment", RuleId(0x7680abd0)),
  ("left-angle", RuleId(0x030f286e)),
  ("left-angle1", RuleId(0x2fdc8d8d)),
  ("left-angle2", RuleId(0x2cdc88d4)),
  ("left-anti-join", RuleId(0xf2c72bfe)),
  ("left-brace", RuleId(0x894f5ee2)),
  ("left-bracket", RuleId(0x3751c859)),
  ("left-join", RuleId(0xadbef3a7)),
  ("left-parenthesis", RuleId(0x943016d1)),
  ("left-semi-join", RuleId(0x4af31ed4)),
  ("less-than", RuleId(0x6c0762f0)),
  ("less-than-equal", RuleId(0xdeecdc0d)),
  ("list-separator", RuleId(0x769235fb)),
  ("literal", RuleId(0xecb9d8e4)),
  ("load-rpl", RuleId(0xf98999f6)),
  ("logic-operator", RuleId(0x945b7804)),
  ("ls-rpl", RuleId(0x344e716b)),
  ("map", RuleId(0xdfa2efb1)),
  ("mapping", RuleId(0x26045d85)),
  ("match-arm", RuleId(0xacb63481)),
  ("match-expression", RuleId(0x4624f5f7)),
  ("matrix", RuleId(0x15c2f8ec)),
  ("matrix-column", RuleId(0xad9a75b9)),
  ("matrix-comprehension", RuleId(0x9e84bbad)),
  ("matrix-end", RuleId(0x773af4a0)),
  ("matrix-multiply", RuleId(0x031bc15b)),
  ("matrix-operator", RuleId(0x676e3633)),
  ("matrix-row", RuleId(0x996fe1d9)),
  ("matrix-solve", RuleId(0xd55a025e)),
  ("matrix-start", RuleId(0x651a7c61)),
  ("mech-code", RuleId(0x392e124a)),
  ("mech-code-alt", RuleId(0xc70ac43c)),
  ("mechdown-list", RuleId(0x86a17f57)),
  ("mechdown-table", RuleId(0xedeb6ce1)),
  ("mechdown-table-header", RuleId(0xab7982a1)),
  ("mechdown-table-no-header", RuleId(0xffbd1bd3)),
  ("mechdown-table-row", RuleId(0x62b41200)),
  ("mechdown-table-with-header", RuleId(0x0b0c29f2)),
  ("micro-mika", RuleId(0x37e1e41c)),
  ("mika", RuleId(0x867d2e5b)),
  ("mika-arm-left", RuleId(0x013ac422)),
  ("mika-arm-right", RuleId(0xac428a0f)),
  ("mika-expression-inner", RuleId(0xe0ace931)),
  ("mika-eye-left", RuleId(0x6d1d2247)),
  ("mika-eye-right", RuleId(0x38745b78)),
  ("mika-nose", RuleId(0xd70bb539)),
  ("mika-section", RuleId(0x3e17623d)),
  ("mika-section-close", RuleId(0x4cf15934)),
  ("mika-section-open", RuleId(0x41251038)),
  ("mini-mika", RuleId(0xa0f642ab)),
  ("module-export-sigil", RuleId(0xc62f20bb)),
  ("module-import", RuleId(0xcd8e7a0d)),
  ("module-import-alias", RuleId(0x74b3f62c)),
  ("module-import-alias-path", RuleId(0x218cf9f4)),
  ("module-import-alias-segment", RuleId(0xf439127c)),
  ("module-import-context-alias", RuleId(0xd8c35316)),
  ("module-import-end", RuleId(0xa7907a45)),
  ("module-import-intrinsic-segment", RuleId(0xdfd1abcf)),
  ("module-import-name-segment", RuleId(0xf485996b)),
  ("module-import-path", RuleId(0xb14e9f87)),
  ("module-import-path-segment", RuleId(0x8be71fc7)),
  ("module-import-sigil", RuleId(0x8ec63758)),
  ("module-import-value-alias", RuleId(0x00ac2d10)),
  ("module-only-import", RuleId(0x0017367c)),
  ("module-root", RuleId(0x7361f448)),
  ("module-suffix-import", RuleId(0xec579f41)),
  ("modulus", RuleId(0x5e58361a)),
  ("mul-assign-operator", RuleId(0xaa375e08)),
  ("mul-div-operator", RuleId(0x4be4a9ae)),
  ("multiply", RuleId(0xff942445)),
  ("nbsp", RuleId(0xf83516fa)),
  ("negate", RuleId(0x757cbb5b)),
  ("negate-factor", RuleId(0x37d7aba5)),
  ("new-line", RuleId(0xdfeb2466)),
  ("new-line-char", RuleId(0x8fe26749)),
  ("newline-indent", RuleId(0xe2af48d8)),
  ("no-alignment", RuleId(0x9cc2288e)),
  ("not", RuleId(0x29b19c8a)),
  ("not-element-of", RuleId(0x3064730f)),
  ("not-equal", RuleId(0x6dcf428f)),
  ("not-factor", RuleId(0x699126c6)),
  ("not-mech-code", RuleId(0x499cfa6a)),
  ("number", RuleId(0x1bd670a0)),
  ("octal-literal", RuleId(0x853e83d2)),
  ("op-assign", RuleId(0xf528b38c)),
  ("op-assign-operator", RuleId(0xe895ccd3)),
  ("option-map", RuleId(0x1683736b)),
  ("option-mapping", RuleId(0x76ce32df)),
  ("option-value", RuleId(0x0baa2318)),
  ("or", RuleId(0x5d342984)),
  ("ordered-list", RuleId(0xf8e6880d)),
  ("ordered-list-item", RuleId(0x3bef3dcf)),
  ("output-operator", RuleId(0x5c28bc8b)),
  ("output-operator-a", RuleId(0x34e84b49)),
  ("output-operator-u", RuleId(0x28e83865)),
  ("paragraph", RuleId(0x8ffa6139)),
  ("paragraph-element", RuleId(0x72aba8e4)),
  ("paragraph-newline", RuleId(0xaa368fb8)),
  ("paragraph-text", RuleId(0x6195191b)),
  ("parenthetical-term", RuleId(0xe4e59e64)),
  ("parse", RuleId(0x423b42ec)),
  ("parse-grammar", RuleId(0x21b8fb74)),
  ("parse-mech", RuleId(0xaf070dc6)),
  ("parse-repl-command", RuleId(0xf69fc5f6)),
  ("pattern", RuleId(0x873d0129)),
  ("pattern-array", RuleId(0x39a90801)),
  ("pattern-array-item", RuleId(0x06d8e89b)),
  ("pattern-array-token", RuleId(0xc551b941)),
  ("pattern-atom-struct", RuleId(0xe2a98d89)),
  ("pattern-tuple", RuleId(0xd3fe175a)),
  ("pattern-tuple-struct", RuleId(0xcd363350)),
  ("percent", RuleId(0x75f9fa5a)),
  ("period", RuleId(0x99c94704)),
  ("plan-rpl", RuleId(0x178922b1)),
  ("plus", RuleId(0xc4adc675)),
  ("power", RuleId(0xf54f2346)),
  ("power-operator", RuleId(0x9c9c7659)),
  ("prefixed-context-path", RuleId(0xb9c9a952)),
  ("profile-rpl", RuleId(0xb992f1cd)),
  ("program", RuleId(0x3d8466cb)),
  ("prompt", RuleId(0xdfe6493b)),
  ("prompt-sigil", RuleId(0xd534900e)),
  ("proper-subset", RuleId(0x60cb584c)),
  ("proper-superset", RuleId(0x1224ecfb)),
  ("punctuation", RuleId(0xbe3ef1b7)),
  ("question", RuleId(0x06d5a845)),
  ("question-block", RuleId(0x0afa1bc9)),
  ("question-sigil", RuleId(0xf6b0a150)),
  ("quit-rpl", RuleId(0x02b465b5)),
  ("quote", RuleId(0xb2887bd7)),
  ("quote-block", RuleId(0x851a4eeb)),
  ("quote-sigil", RuleId(0x71eb3c62)),
  ("range-exclusive", RuleId(0xba78cbe3)),
  ("range-expression", RuleId(0xc4fce9b3)),
  ("range-inclusive", RuleId(0x97191a09)),
  ("range-operator", RuleId(0xca143fc5)),
  ("range-subscript", RuleId(0xa81786c2)),
  ("rational-literal", RuleId(0x9ed5a559)),
  ("raw-hyperlink", RuleId(0x6b43470e)),
  ("raw-string", RuleId(0x82e2f075)),
  ("raw-subtract", RuleId(0x6f9c5a48)),
  ("raw-text", RuleId(0xb5808b43)),
  ("real-number", RuleId(0xb7a5a861)),
  ("record", RuleId(0x593058cc)),
  ("reference", RuleId(0x5a81f39a)),
  ("regular-table", RuleId(0x6d98dd22)),
  ("relative-source-import-specifier", RuleId(0x48147430)),
  ("repl-identifier", RuleId(0xec2868a0)),
  ("right-alignment", RuleId(0x992d85ad)),
  ("right-angle", RuleId(0x9eaa795b)),
  ("right-angle1", RuleId(0x305d21de)),
  ("right-angle2", RuleId(0x2f5d204b)),
  ("right-brace", RuleId(0xc31f2933)),
  ("right-bracket", RuleId(0xacb1a1b4)),
  ("right-join", RuleId(0xe40683c8)),
  ("right-parenthesis", RuleId(0xfcbd484c)),
  ("row-separator", RuleId(0x767c3ee5)),
  ("save-rpl", RuleId(0x869d77cb)),
  ("scientific-literal", RuleId(0xd8d7e236)),
  ("section", RuleId(0xfcdd0ccc)),
  ("section-element", RuleId(0x520f1845)),
  ("section-reference", RuleId(0x5f8853d8)),
  ("section-sigil", RuleId(0x30175a21)),
  ("select-all", RuleId(0x3eda43b3)),
  ("semicolon", RuleId(0x0415d01e)),
  ("send-operator", RuleId(0xe17e27f6)),
  ("set", RuleId(0xc6270703)),
  ("set-comprehension", RuleId(0xb27d4a02)),
  ("set-operator", RuleId(0x2f3b5e5a)),
  ("slash", RuleId(0xcb73e8ea)),
  ("slice", RuleId(0x6789b051)),
  ("slice-ref", RuleId(0x6076ee69)),
  ("source-import-specifier", RuleId(0x23a2f69b)),
  ("source-import-tail", RuleId(0xcec6e017)),
  ("source-import-uri-scheme", RuleId(0xd05ccd33)),
  ("source-mec-path", RuleId(0x0eaeb222)),
  ("source-mec-path-wildcard-suffix", RuleId(0x5d51d6bb)),
  ("source-path-component", RuleId(0x096d5b3a)),
  ("source-path-component-token", RuleId(0xe9cba9a2)),
  ("space", RuleId(0x3553e285)),
  ("space-tab", RuleId(0xddfa1f07)),
  ("space-tab0", RuleId(0xa7bf2395)),
  ("space-tab1", RuleId(0xa6bf2202)),
  ("spaced-subtract", RuleId(0x9008603e)),
  ("spread-operator", RuleId(0xbd1421c1)),
  ("spread-operator-a", RuleId(0xd244fa7f)),
  ("spread-operator-u", RuleId(0xde450d63)),
  ("statement", RuleId(0xfb8d6d16)),
  ("statement-separator", RuleId(0x6d3cbe08)),
  ("step-rpl", RuleId(0x67b402b8)),
  ("strict-equal", RuleId(0x55633ca1)),
  ("strict-not-equal", RuleId(0x9e40ed09)),
  ("strike-sigil", RuleId(0x9f47a3da)),
  ("strikethrough", RuleId(0x1eb28c44)),
  ("string", RuleId(0x17c16538)),
  ("strong", RuleId(0xc51f5d7a)),
  ("strong-sigil", RuleId(0x52eb6537)),
  ("structure", RuleId(0x6f5525c6)),
  ("sub-assign-operator", RuleId(0x8d7f686c)),
  ("sublist", RuleId(0x8b09b311)),
  ("subscript", RuleId(0xb27bbc7a)),
  ("subset", RuleId(0x4dab1c73)),
  ("subtitle", RuleId(0x115bfcb9)),
  ("subtract", RuleId(0x42c1a561)),
  ("success-block", RuleId(0xfd7fb978)),
  ("success-check-sigil", RuleId(0xc9b89aac)),
  ("success-sigil", RuleId(0x2ea879dd)),
  ("superset", RuleId(0x0022ac1c)),
  ("swizzle-subscript", RuleId(0xf2757b45)),
  ("symbol", RuleId(0xf3fb51d1)),
  ("symbols-rpl", RuleId(0x96497345)),
  ("symmetric-difference", RuleId(0x448cc168)),
  ("synth-operator", RuleId(0x5b70298c)),
  ("tab", RuleId(0x98f72e4c)),
  ("table", RuleId(0x4a9c9bdf)),
  ("table-column", RuleId(0xf07e4b8c)),
  ("table-end", RuleId(0x683547b3)),
  ("table-header", RuleId(0xff420217)),
  ("table-horz", RuleId(0x085c40c7)),
  ("table-operator", RuleId(0x1e434d06)),
  ("table-row", RuleId(0x283dfcaa)),
  ("table-row2", RuleId(0xf194a348)),
  ("table-separator", RuleId(0xcb5b4979)),
  ("table-start", RuleId(0xa8301216)),
  ("table-top", RuleId(0x4e2be981)),
  ("tag", RuleId(0x95f72993)),
  ("text", RuleId(0xbde64e3e)),
  ("thematic-break", RuleId(0xaa6ec998)),
  ("thin-space", RuleId(0x9f3729bd)),
  ("tilde", RuleId(0xff1023a9)),
  ("tilde-codeblock-sigil", RuleId(0xa03569e3)),
  ("title", RuleId(0x9865b509)),
  ("title-front-matter", RuleId(0xc0534f8b)),
  ("transition-operator", RuleId(0x6dc19135)),
  ("transition-operator-a", RuleId(0xd6b9c20b)),
  ("transition-operator-u", RuleId(0xeab9e187)),
  ("transpose", RuleId(0xc03183c0)),
  ("true-literal", RuleId(0x2a7b8757)),
  ("tuple", RuleId(0x92722331)),
  ("tuple-destructure", RuleId(0x6e0c5c64)),
  ("tuple-struct", RuleId(0xdd5df0e5)),
  ("typed-integer", RuleId(0x463d56ac)),
  ("ul-subtitle", RuleId(0x2d8e6d27)),
  ("unchecked-item", RuleId(0xa4ce1a79)),
  ("underline", RuleId(0xe582347f)),
  ("underline-sigil", RuleId(0x4f35466a)),
  ("underscore", RuleId(0x34fdbabd)),
  ("underscore-digit", RuleId(0xe34f0295)),
  ("union-op", RuleId(0x5655b904)),
  ("unordered-list", RuleId(0xb41d2946)),
  ("unordered-list-item", RuleId(0xf5586d6a)),
  ("untyped-integer", RuleId(0x119a93f5)),
  ("untyped-real-number", RuleId(0xc01271f1)),
  ("uri-scheme-part", RuleId(0x085c8a27)),
  ("uri-source-import-specifier", RuleId(0x3c8a2e90)),
  ("utf8-string", RuleId(0x04a99a40)),
  ("var", RuleId(0x8a25e7be)),
  ("variable-assign", RuleId(0x562b9e49)),
  ("variable-define", RuleId(0x47193f33)),
  ("warning-block", RuleId(0x6ffb83a3)),
  ("warning-sigil", RuleId(0xba3ac17a)),
  ("whitespace", RuleId(0xd240f720)),
  ("whitespace0", RuleId(0x0c44ee30)),
  ("whitespace1", RuleId(0x0d44efc3)),
  ("whos-rpl", RuleId(0x71a8c9b7)),
  ("wildcard", RuleId(0xb54fab6f)),
  ("ws0e", RuleId(0x02f8cfc8)),
  ("ws1e", RuleId(0xe6f6651d)),
  ("xor", RuleId(0xcc6bdb7e)),
];

pub mod rules {
  use crate::document::RuleId;

  pub const ABSOLUTE_SOURCE_IMPORT_SPECIFIER: RuleId = RuleId(0x37410b35);
  pub const ABSTRACT_EL: RuleId = RuleId(0xed0e65f3);
  pub const ABSTRACT_SIGIL: RuleId = RuleId(0xae0225c6);
  pub const ACTIVATION_ARM: RuleId = RuleId(0xc5b32bea);
  pub const ACTIVATION_SCOPE: RuleId = RuleId(0x453304d4);
  pub const ADD: RuleId = RuleId(0x3b391274);
  pub const ADD_ASSIGN_OPERATOR: RuleId = RuleId(0x77538e15);
  pub const ADD_SUB_OPERATOR: RuleId = RuleId(0x2b923e96);
  pub const ALIASED_ITEM_IMPORT: RuleId = RuleId(0xb7bcdda4);
  pub const ALIGNMENT_SEPARATOR: RuleId = RuleId(0xaea70f10);
  pub const ALPHA: RuleId = RuleId(0x5d8b6dab);
  pub const ALPHA_TOKEN: RuleId = RuleId(0xab243f87);
  pub const ALPHANUMERIC: RuleId = RuleId(0x0b09b0d6);
  pub const AMPERSAND: RuleId = RuleId(0xebb16704);
  pub const AND: RuleId = RuleId(0x0f29c2a6);
  pub const ANY: RuleId = RuleId(0x2c29f04d);
  pub const ANY_TOKEN: RuleId = RuleId(0xd91e0675);
  pub const APOSTROPHE: RuleId = RuleId(0xdff65064);
  pub const ARGUMENT_LIST: RuleId = RuleId(0xc1b3b46d);
  pub const ASSIGN_OPERATOR: RuleId = RuleId(0x688f2a93);
  pub const ASTERISK: RuleId = RuleId(0xef8a6081);
  pub const ASYNC_TRANSITION_OPERATOR: RuleId = RuleId(0xa37fbb00);
  pub const AT: RuleId = RuleId(0x57251588);
  pub const ATOM: RuleId = RuleId(0x037448d8);
  pub const BACKSLASH: RuleId = RuleId(0x6f17fb77);
  pub const BAR: RuleId = RuleId(0x76b77d1a);
  pub const BARE_SOURCE_IMPORT_SPECIFIER: RuleId = RuleId(0x49acf5a0);
  pub const BINARY_LITERAL: RuleId = RuleId(0x521c71d2);
  pub const BINDING: RuleId = RuleId(0xc5955a62);
  pub const BLANK_LINE: RuleId = RuleId(0x8abba276);
  pub const BODY: RuleId = RuleId(0xdbaa7975);
  pub const BOOLEAN: RuleId = RuleId(0x65f46ebf);
  pub const BOX_BL: RuleId = RuleId(0xc5e59dc3);
  pub const BOX_BL_BOLD: RuleId = RuleId(0x4d5fd5cd);
  pub const BOX_BL_ROUND: RuleId = RuleId(0xba2cab7a);
  pub const BOX_BR: RuleId = RuleId(0xc3e59a9d);
  pub const BOX_BR_BOLD: RuleId = RuleId(0xc9640063);
  pub const BOX_BR_ROUND: RuleId = RuleId(0xb4d94328);
  pub const BOX_CROSS: RuleId = RuleId(0xb51d1803);
  pub const BOX_DRAWING_CHAR: RuleId = RuleId(0x8ee2110c);
  pub const BOX_DRAWING_EMOJI: RuleId = RuleId(0xb0ab856e);
  pub const BOX_HORZ: RuleId = RuleId(0x3bbd73d4);
  pub const BOX_T_BOTTOM: RuleId = RuleId(0x5e60b5b1);
  pub const BOX_T_LEFT: RuleId = RuleId(0x33aad62f);
  pub const BOX_T_RIGHT: RuleId = RuleId(0xc046cc20);
  pub const BOX_T_TOP: RuleId = RuleId(0x359df285);
  pub const BOX_TL: RuleId = RuleId(0xc9ff3bf5);
  pub const BOX_TL_BOLD: RuleId = RuleId(0xc7baef3b);
  pub const BOX_TL_ROUND: RuleId = RuleId(0x73142f90);
  pub const BOX_TR: RuleId = RuleId(0xbbff25eb);
  pub const BOX_TR_BOLD: RuleId = RuleId(0x58c0be35);
  pub const BOX_TR_ROUND: RuleId = RuleId(0x1f0a5952);
  pub const BOX_VERT: RuleId = RuleId(0x7757ebaa);
  pub const BOX_VERT_BOLD: RuleId = RuleId(0x74980a36);
  pub const BRACE_SUBSCRIPT: RuleId = RuleId(0x263793c4);
  pub const BRACKET_SUBSCRIPT: RuleId = RuleId(0xfcb4c29b);
  pub const CALL_ARG: RuleId = RuleId(0x77ead188);
  pub const CALL_ARG_WITH_BINDING: RuleId = RuleId(0x8be85223);
  pub const CARET: RuleId = RuleId(0x7aef16a8);
  pub const CARRIAGE_RETURN: RuleId = RuleId(0xdde3fe8c);
  pub const CARRIAGE_RETURN_NEW_LINE: RuleId = RuleId(0xff2066c8);
  pub const CD_RPL: RuleId = RuleId(0x8c2f9121);
  pub const CENTER_ALIGNMENT: RuleId = RuleId(0xb4b26da4);
  pub const CHECK_LIST: RuleId = RuleId(0x799fadea);
  pub const CHECK_LIST_ITEM: RuleId = RuleId(0x0846ce66);
  pub const CHECK_MARK: RuleId = RuleId(0xc1f3e14f);
  pub const CHECKED_ITEM: RuleId = RuleId(0x20377dea);
  pub const CITATION: RuleId = RuleId(0xd92e6394);
  pub const CLC_RPL: RuleId = RuleId(0x010d7744);
  pub const CLEAR_RPL: RuleId = RuleId(0xae623141);
  pub const CODE_BLOCK: RuleId = RuleId(0x47edebd4);
  pub const CODE_RPL: RuleId = RuleId(0xb1801777);
  pub const CODE_TERMINAL: RuleId = RuleId(0x7019f177);
  pub const CODEBLOCK_SIGIL: RuleId = RuleId(0x7b2e0578);
  pub const COLON: RuleId = RuleId(0x497e753c);
  pub const COMMA: RuleId = RuleId(0xbffa8578);
  pub const COMMENT: RuleId = RuleId(0x67a6c45e);
  pub const COMMENT_SIGIL: RuleId = RuleId(0xbf72dce3);
  pub const COMPARISON_OPERATOR: RuleId = RuleId(0xb678194b);
  pub const COMPLEMENT: RuleId = RuleId(0x5282471d);
  pub const COMPLEX_NUMBER: RuleId = RuleId(0x6a69227b);
  pub const COMPREHENSION_QUALIFIER: RuleId = RuleId(0xe50e092e);
  pub const CONTEXT_ADDRESS_PATH: RuleId = RuleId(0x94fb8e83);
  pub const CONTEXT_ADDRESS_PATH_TOKEN: RuleId = RuleId(0x19a621df);
  pub const CONTEXT_BASE_CONTEXT: RuleId = RuleId(0x33efe980);
  pub const CONTEXT_BASE_RESOURCE_URI: RuleId = RuleId(0xfa7d8ac6);
  pub const CONTEXT_CAPABILITY_DECLARATION: RuleId = RuleId(0x122a0454);
  pub const CONTEXT_CAPABILITY_PATH: RuleId = RuleId(0x9d4416c7);
  pub const CONTEXT_CAPABILITY_PATH_TOKEN: RuleId = RuleId(0xc65e4f0b);
  pub const CONTEXT_CAPABILITY_SCOPE: RuleId = RuleId(0x7fb1bcc4);
  pub const CONTEXT_DECLARATION: RuleId = RuleId(0xc5776b35);
  pub const CONTEXT_IMPORT_ALIAS_SEGMENT: RuleId = RuleId(0xdf58baa7);
  pub const CONTEXT_SEND: RuleId = RuleId(0xdab0da55);
  pub const CROSS: RuleId = RuleId(0x29f5189b);
  pub const CROSS_PRODUCT: RuleId = RuleId(0xbca81ce1);
  pub const DASH: RuleId = RuleId(0x0179def5);
  pub const DECIMAL_LITERAL: RuleId = RuleId(0x04e47722);
  pub const DEFINE_OPERATOR: RuleId = RuleId(0xdeffa145);
  pub const DIFFERENCE: RuleId = RuleId(0xea8f6e42);
  pub const DIGIT: RuleId = RuleId(0x885c8a56);
  pub const DIGIT_SEQUENCE: RuleId = RuleId(0x70208d0c);
  pub const DIGIT_TOKEN: RuleId = RuleId(0x416fb636);
  pub const DIV_ASSIGN_OPERATOR: RuleId = RuleId(0x2121da89);
  pub const DIVIDE: RuleId = RuleId(0x61526270);
  pub const DOCS_RPL: RuleId = RuleId(0x3f0b9bf1);
  pub const DOLLAR: RuleId = RuleId(0xf364bd3f);
  pub const DOT_PRODUCT: RuleId = RuleId(0x6bf9d186);
  pub const DOT_SUBSCRIPT: RuleId = RuleId(0xad92070c);
  pub const DOT_SUBSCRIPT_INT: RuleId = RuleId(0x40db609c);
  pub const ELEMENT_OF: RuleId = RuleId(0x8284dd6f);
  pub const EMOJI: RuleId = RuleId(0x4a90ef3d);
  pub const EMOJI_GRAPHEME: RuleId = RuleId(0xce4c615d);
  pub const EMPHASIS: RuleId = RuleId(0x5ff3a9f7);
  pub const EMPHASIS_SIGIL: RuleId = RuleId(0xfdce2882);
  pub const EMPTY: RuleId = RuleId(0x18a7beee);
  pub const EMPTY_MAP: RuleId = RuleId(0xe8a9c9f5);
  pub const EMPTY_PARAGRAPH: RuleId = RuleId(0xe7a8e3ad);
  pub const EMPTY_SET: RuleId = RuleId(0xa0d9b56f);
  pub const ENGLISH_FALSE_LITERAL: RuleId = RuleId(0x04f87d33);
  pub const ENGLISH_TRUE_LITERAL: RuleId = RuleId(0xbcec4d60);
  pub const ENUM_DEFINE: RuleId = RuleId(0xa8bdcda4);
  pub const ENUM_SEPARATOR: RuleId = RuleId(0x57266112);
  pub const ENUM_VARIANT: RuleId = RuleId(0xfec00678);
  pub const ENUM_VARIANT_INLINE_KIND: RuleId = RuleId(0xea7ed22f);
  pub const ENUM_VARIANT_KIND: RuleId = RuleId(0x2b5cf505);
  pub const EQUAL: RuleId = RuleId(0x2f7508ef);
  pub const EQUAL_TO: RuleId = RuleId(0x00e4debb);
  pub const EQUATION: RuleId = RuleId(0x943adfd3);
  pub const EQUATION_SIGIL: RuleId = RuleId(0x61878026);
  pub const ERROR_ALT_SIGIL: RuleId = RuleId(0x85d7a022);
  pub const ERROR_BLOCK: RuleId = RuleId(0x44314f6d);
  pub const ERROR_SIGIL: RuleId = RuleId(0x41a3f3f4);
  pub const ESCAPED_CHAR: RuleId = RuleId(0x7df2caa9);
  pub const EVAL_INLINE_MECH_CODE: RuleId = RuleId(0x5a5b75d7);
  pub const EXCLAMATION: RuleId = RuleId(0xc234a5d6);
  pub const EXP_ASSIGN_OPERATOR: RuleId = RuleId(0xa17bd969);
  pub const EXPORT_DECLARATION: RuleId = RuleId(0x34842f0a);
  pub const EXPRESSION: RuleId = RuleId(0xcf15afeb);
  pub const FACTOR: RuleId = RuleId(0x5c8ff3a6);
  pub const FALSE_LITERAL: RuleId = RuleId(0x03c29ce6);
  pub const FANCY_TABLE: RuleId = RuleId(0xdaf03453);
  pub const FANCY_TABLE_HEADER: RuleId = RuleId(0xa4e5b5a3);
  pub const FIELD: RuleId = RuleId(0x67826267);
  pub const FIGURE_ITEM: RuleId = RuleId(0xd09598c3);
  pub const FIGURES: RuleId = RuleId(0x33a20f0e);
  pub const FIGURES_ROW: RuleId = RuleId(0xbf987a37);
  pub const FLOAT: RuleId = RuleId(0xa6c45d85);
  pub const FLOAT_DECIMAL_START: RuleId = RuleId(0xb3b229f6);
  pub const FLOAT_FULL: RuleId = RuleId(0x2316b35d);
  pub const FLOAT_LEFT: RuleId = RuleId(0xd95b4809);
  pub const FLOAT_LITERAL: RuleId = RuleId(0xc3d7b977);
  pub const FLOAT_RIGHT: RuleId = RuleId(0x0d2337aa);
  pub const FLOAT_SIGIL: RuleId = RuleId(0x8d57ea10);
  pub const FOOTNOTE: RuleId = RuleId(0xdda62331);
  pub const FOOTNOTE_PREFIX: RuleId = RuleId(0x12f25b90);
  pub const FOOTNOTE_REFERENCE: RuleId = RuleId(0x25ddcf69);
  pub const FORBIDDEN_EMOJI: RuleId = RuleId(0xa330fe15);
  pub const FORMULA: RuleId = RuleId(0x798fbc5d);
  pub const FORMULA_SUBSCRIPT: RuleId = RuleId(0x3b3f7c19);
  pub const FSM: RuleId = RuleId(0xcbd59f49);
  pub const FSM_ARGS: RuleId = RuleId(0x7aa190c1);
  pub const FSM_ARM: RuleId = RuleId(0x4ee1df36);
  pub const FSM_ASYNC_TRANSITION: RuleId = RuleId(0x0bdf7a88);
  pub const FSM_BLOCK_TRANSITION: RuleId = RuleId(0x8f392bab);
  pub const FSM_COMMENT_ARM: RuleId = RuleId(0x7aef4356);
  pub const FSM_DECLARE: RuleId = RuleId(0xd4b2abc2);
  pub const FSM_GUARD: RuleId = RuleId(0x6a7c9ca1);
  pub const FSM_GUARD_ARM: RuleId = RuleId(0x83c43a4e);
  pub const FSM_IMPLEMENTATION: RuleId = RuleId(0x475b3a3e);
  pub const FSM_INSTANCE: RuleId = RuleId(0x9d5b7817);
  pub const FSM_OUTPUT: RuleId = RuleId(0x68e572c1);
  pub const FSM_PIPE: RuleId = RuleId(0x8ecde250);
  pub const FSM_SPECIFICATION: RuleId = RuleId(0x2297b501);
  pub const FSM_STATE_DEFINITION: RuleId = RuleId(0xb256027d);
  pub const FSM_STATE_DEFINITION_VARIABLES: RuleId = RuleId(0xb2347839);
  pub const FSM_STATE_TRANSITION: RuleId = RuleId(0x717cb24f);
  pub const FSM_STATEMENT_TRANSITION: RuleId = RuleId(0x665eb907);
  pub const FSM_TRANSITION: RuleId = RuleId(0xbba72217);
  pub const FSM_VALUE: RuleId = RuleId(0xb3b95dc5);
  pub const FULL_JOIN: RuleId = RuleId(0x112a81a3);
  pub const FUNCTION_ARG: RuleId = RuleId(0x8409bba8);
  pub const FUNCTION_CALL: RuleId = RuleId(0xfcbdb56c);
  pub const FUNCTION_DEFINE: RuleId = RuleId(0xb0b7ceff);
  pub const FUNCTION_DEFINE_MATCH_ARMS: RuleId = RuleId(0x92fe8211);
  pub const FUNCTION_DEFINE_STATEMENTS: RuleId = RuleId(0x0e0bf554);
  pub const FUNCTION_MATCH_ARM: RuleId = RuleId(0x15839c42);
  pub const FUNCTION_OUT_ARG: RuleId = RuleId(0x9177c6a5);
  pub const FUNCTION_OUT_ARGS: RuleId = RuleId(0xd58e02e2);
  pub const GEN_OPERATOR: RuleId = RuleId(0xa54f7858);
  pub const GENERATOR: RuleId = RuleId(0x6eec35c2);
  pub const GENERATOR_ARROW: RuleId = RuleId(0x1b9c235e);
  pub const GENERATOR_ARROW_U: RuleId = RuleId(0x87d60d34);
  pub const GRAMMAR: RuleId = RuleId(0x4b536ffe);
  pub const GRAMMAR_DEFINITION: RuleId = RuleId(0x4f332cec);
  pub const GRAMMAR_EXPRESSION: RuleId = RuleId(0xca4269bf);
  pub const GRAMMAR_FACTOR: RuleId = RuleId(0x27c3d872);
  pub const GRAMMAR_GROUP: RuleId = RuleId(0xfce872a0);
  pub const GRAMMAR_IDENTIFIER: RuleId = RuleId(0x759b4f92);
  pub const GRAMMAR_LIST: RuleId = RuleId(0xfc4ab7f5);
  pub const GRAMMAR_NOT: RuleId = RuleId(0x5a19eb5e);
  pub const GRAMMAR_OPTIONAL: RuleId = RuleId(0xec6546b5);
  pub const GRAMMAR_PEEK: RuleId = RuleId(0x9b771a32);
  pub const GRAMMAR_RANGE: RuleId = RuleId(0xf923b3f6);
  pub const GRAMMAR_REPEAT0: RuleId = RuleId(0xd3934312);
  pub const GRAMMAR_REPEAT1: RuleId = RuleId(0xd49344a5);
  pub const GRAMMAR_RULE: RuleId = RuleId(0x7cb77e2f);
  pub const GRAMMAR_TERM: RuleId = RuleId(0x8aa0e583);
  pub const GRAMMAR_TERMINAL: RuleId = RuleId(0x36f7809d);
  pub const GRAMMAR_TERMINAL_TOKEN: RuleId = RuleId(0xdc8237a5);
  pub const GRAVE: RuleId = RuleId(0x9068bb32);
  pub const GRAVE_CODEBLOCK_SIGIL: RuleId = RuleId(0x7d321eb0);
  pub const GREATER_THAN: RuleId = RuleId(0x57a89e97);
  pub const GREATER_THAN_EQUAL: RuleId = RuleId(0x0ac93612);
  pub const GROUPING_SYMBOL: RuleId = RuleId(0x4ec76763);
  pub const GUARD_OPERATOR: RuleId = RuleId(0x11c1cce1);
  pub const HASHTAG: RuleId = RuleId(0x580caca7);
  pub const HEADER_FIELD: RuleId = RuleId(0x8fed2869);
  pub const HELP_RPL: RuleId = RuleId(0xbdc66d79);
  pub const HEXADECIMAL_LITERAL: RuleId = RuleId(0x38b815ae);
  pub const HIGHLIGHT: RuleId = RuleId(0x1c9ff127);
  pub const HIGHLIGHT_SIGIL: RuleId = RuleId(0x309033d2);
  pub const HTTP_PREFIX: RuleId = RuleId(0x185c9154);
  pub const HYPERLINK: RuleId = RuleId(0xb189102d);
  pub const IDEA_BLOCK: RuleId = RuleId(0x88ff8e92);
  pub const IDEA_SIGIL: RuleId = RuleId(0xe8bf28d7);
  pub const IDENTIFIER: RuleId = RuleId(0x28a5a83e);
  pub const IDENTIFIER_PATH_SEGMENT: RuleId = RuleId(0xa5b1745e);
  pub const IDENTIFIER_PATH_SEGMENT_EMOJI: RuleId = RuleId(0xa36539c1);
  pub const IDENTIFIER_SYMBOL: RuleId = RuleId(0xb3aeb85d);
  pub const IMG: RuleId = RuleId(0x84e72504);
  pub const IMG_PREFIX: RuleId = RuleId(0xef2025ff);
  pub const IMPORT_ALIAS_OPERATOR: RuleId = RuleId(0xddb49ce8);
  pub const IMPORT_DECLARATION: RuleId = RuleId(0x38f7ba9d);
  pub const IMPORT_GROUP_ITEM: RuleId = RuleId(0x583a450e);
  pub const IMPORT_GROUP_ITEMS: RuleId = RuleId(0x60bb63c7);
  pub const IMPORT_GROUP_SEPARATOR: RuleId = RuleId(0xcdfd49c4);
  pub const INFO_BLOCK: RuleId = RuleId(0x36c95509);
  pub const INFO_SIGIL: RuleId = RuleId(0x227e4790);
  pub const INLINE_CODE: RuleId = RuleId(0xcced3fa2);
  pub const INLINE_EQUATION: RuleId = RuleId(0xc6764779);
  pub const INLINE_MECH_CODE: RuleId = RuleId(0x833b5950);
  pub const INLINE_PARAGRAPH: RuleId = RuleId(0xbc63fc47);
  pub const INLINE_TABLE: RuleId = RuleId(0x11f04f9d);
  pub const INLINE_TABLE_HEADER: RuleId = RuleId(0x162ff185);
  pub const INLINE_TABLE_ROW: RuleId = RuleId(0x4f6fdcb4);
  pub const INTEGER_LITERAL: RuleId = RuleId(0xff5fa0b7);
  pub const INTERSECTION: RuleId = RuleId(0x6be1e5c8);
  pub const INVARIANT_DEFINE: RuleId = RuleId(0x8a084469);
  pub const JOIN: RuleId = RuleId(0xc922bc79);
  pub const KIND: RuleId = RuleId(0xd913e243);
  pub const KIND_ANNOTATION: RuleId = RuleId(0x879cbcf3);
  pub const KIND_ANY: RuleId = RuleId(0xb1c044fc);
  pub const KIND_ATOM: RuleId = RuleId(0x530824e7);
  pub const KIND_DEFINE: RuleId = RuleId(0x42029f05);
  pub const KIND_EMPTY: RuleId = RuleId(0x52549b1f);
  pub const KIND_KIND: RuleId = RuleId(0x86b3944c);
  pub const KIND_MAP: RuleId = RuleId(0x03c3eed4);
  pub const KIND_MATRIX: RuleId = RuleId(0x35c4f5e7);
  pub const KIND_RECORD: RuleId = RuleId(0x4b6afb9f);
  pub const KIND_SCALAR: RuleId = RuleId(0xed70de7a);
  pub const KIND_SET: RuleId = RuleId(0xad43176a);
  pub const KIND_TABLE: RuleId = RuleId(0xbadb583e);
  pub const KIND_TUPLE: RuleId = RuleId(0x45e1a60c);
  pub const KIND_WITH_OPTION: RuleId = RuleId(0xd0ad1bd0);
  pub const L1: RuleId = RuleId(0x18317e4e);
  pub const L2: RuleId = RuleId(0x17317cbb);
  pub const L3: RuleId = RuleId(0x16317b28);
  pub const L4: RuleId = RuleId(0x1d31862d);
  pub const L5: RuleId = RuleId(0x1c31849a);
  pub const L6: RuleId = RuleId(0x1b318307);
  pub const L7: RuleId = RuleId(0x1a318174);
  pub const LEFT_ALIGNMENT: RuleId = RuleId(0x7680abd0);
  pub const LEFT_ANGLE: RuleId = RuleId(0x030f286e);
  pub const LEFT_ANGLE1: RuleId = RuleId(0x2fdc8d8d);
  pub const LEFT_ANGLE2: RuleId = RuleId(0x2cdc88d4);
  pub const LEFT_ANTI_JOIN: RuleId = RuleId(0xf2c72bfe);
  pub const LEFT_BRACE: RuleId = RuleId(0x894f5ee2);
  pub const LEFT_BRACKET: RuleId = RuleId(0x3751c859);
  pub const LEFT_JOIN: RuleId = RuleId(0xadbef3a7);
  pub const LEFT_PARENTHESIS: RuleId = RuleId(0x943016d1);
  pub const LEFT_SEMI_JOIN: RuleId = RuleId(0x4af31ed4);
  pub const LESS_THAN: RuleId = RuleId(0x6c0762f0);
  pub const LESS_THAN_EQUAL: RuleId = RuleId(0xdeecdc0d);
  pub const LIST_SEPARATOR: RuleId = RuleId(0x769235fb);
  pub const LITERAL: RuleId = RuleId(0xecb9d8e4);
  pub const LOAD_RPL: RuleId = RuleId(0xf98999f6);
  pub const LOGIC_OPERATOR: RuleId = RuleId(0x945b7804);
  pub const LS_RPL: RuleId = RuleId(0x344e716b);
  pub const MAP: RuleId = RuleId(0xdfa2efb1);
  pub const MAPPING: RuleId = RuleId(0x26045d85);
  pub const MATCH_ARM: RuleId = RuleId(0xacb63481);
  pub const MATCH_EXPRESSION: RuleId = RuleId(0x4624f5f7);
  pub const MATRIX: RuleId = RuleId(0x15c2f8ec);
  pub const MATRIX_COLUMN: RuleId = RuleId(0xad9a75b9);
  pub const MATRIX_COMPREHENSION: RuleId = RuleId(0x9e84bbad);
  pub const MATRIX_END: RuleId = RuleId(0x773af4a0);
  pub const MATRIX_MULTIPLY: RuleId = RuleId(0x031bc15b);
  pub const MATRIX_OPERATOR: RuleId = RuleId(0x676e3633);
  pub const MATRIX_ROW: RuleId = RuleId(0x996fe1d9);
  pub const MATRIX_SOLVE: RuleId = RuleId(0xd55a025e);
  pub const MATRIX_START: RuleId = RuleId(0x651a7c61);
  pub const MECH_CODE: RuleId = RuleId(0x392e124a);
  pub const MECH_CODE_ALT: RuleId = RuleId(0xc70ac43c);
  pub const MECHDOWN_LIST: RuleId = RuleId(0x86a17f57);
  pub const MECHDOWN_TABLE: RuleId = RuleId(0xedeb6ce1);
  pub const MECHDOWN_TABLE_HEADER: RuleId = RuleId(0xab7982a1);
  pub const MECHDOWN_TABLE_NO_HEADER: RuleId = RuleId(0xffbd1bd3);
  pub const MECHDOWN_TABLE_ROW: RuleId = RuleId(0x62b41200);
  pub const MECHDOWN_TABLE_WITH_HEADER: RuleId = RuleId(0x0b0c29f2);
  pub const MICRO_MIKA: RuleId = RuleId(0x37e1e41c);
  pub const MIKA: RuleId = RuleId(0x867d2e5b);
  pub const MIKA_ARM_LEFT: RuleId = RuleId(0x013ac422);
  pub const MIKA_ARM_RIGHT: RuleId = RuleId(0xac428a0f);
  pub const MIKA_EXPRESSION_INNER: RuleId = RuleId(0xe0ace931);
  pub const MIKA_EYE_LEFT: RuleId = RuleId(0x6d1d2247);
  pub const MIKA_EYE_RIGHT: RuleId = RuleId(0x38745b78);
  pub const MIKA_NOSE: RuleId = RuleId(0xd70bb539);
  pub const MIKA_SECTION: RuleId = RuleId(0x3e17623d);
  pub const MIKA_SECTION_CLOSE: RuleId = RuleId(0x4cf15934);
  pub const MIKA_SECTION_OPEN: RuleId = RuleId(0x41251038);
  pub const MINI_MIKA: RuleId = RuleId(0xa0f642ab);
  pub const MODULE_EXPORT_SIGIL: RuleId = RuleId(0xc62f20bb);
  pub const MODULE_IMPORT: RuleId = RuleId(0xcd8e7a0d);
  pub const MODULE_IMPORT_ALIAS: RuleId = RuleId(0x74b3f62c);
  pub const MODULE_IMPORT_ALIAS_PATH: RuleId = RuleId(0x218cf9f4);
  pub const MODULE_IMPORT_ALIAS_SEGMENT: RuleId = RuleId(0xf439127c);
  pub const MODULE_IMPORT_CONTEXT_ALIAS: RuleId = RuleId(0xd8c35316);
  pub const MODULE_IMPORT_END: RuleId = RuleId(0xa7907a45);
  pub const MODULE_IMPORT_INTRINSIC_SEGMENT: RuleId = RuleId(0xdfd1abcf);
  pub const MODULE_IMPORT_NAME_SEGMENT: RuleId = RuleId(0xf485996b);
  pub const MODULE_IMPORT_PATH: RuleId = RuleId(0xb14e9f87);
  pub const MODULE_IMPORT_PATH_SEGMENT: RuleId = RuleId(0x8be71fc7);
  pub const MODULE_IMPORT_SIGIL: RuleId = RuleId(0x8ec63758);
  pub const MODULE_IMPORT_VALUE_ALIAS: RuleId = RuleId(0x00ac2d10);
  pub const MODULE_ONLY_IMPORT: RuleId = RuleId(0x0017367c);
  pub const MODULE_ROOT: RuleId = RuleId(0x7361f448);
  pub const MODULE_SUFFIX_IMPORT: RuleId = RuleId(0xec579f41);
  pub const MODULUS: RuleId = RuleId(0x5e58361a);
  pub const MUL_ASSIGN_OPERATOR: RuleId = RuleId(0xaa375e08);
  pub const MUL_DIV_OPERATOR: RuleId = RuleId(0x4be4a9ae);
  pub const MULTIPLY: RuleId = RuleId(0xff942445);
  pub const NBSP: RuleId = RuleId(0xf83516fa);
  pub const NEGATE: RuleId = RuleId(0x757cbb5b);
  pub const NEGATE_FACTOR: RuleId = RuleId(0x37d7aba5);
  pub const NEW_LINE: RuleId = RuleId(0xdfeb2466);
  pub const NEW_LINE_CHAR: RuleId = RuleId(0x8fe26749);
  pub const NEWLINE_INDENT: RuleId = RuleId(0xe2af48d8);
  pub const NO_ALIGNMENT: RuleId = RuleId(0x9cc2288e);
  pub const NOT: RuleId = RuleId(0x29b19c8a);
  pub const NOT_ELEMENT_OF: RuleId = RuleId(0x3064730f);
  pub const NOT_EQUAL: RuleId = RuleId(0x6dcf428f);
  pub const NOT_FACTOR: RuleId = RuleId(0x699126c6);
  pub const NOT_MECH_CODE: RuleId = RuleId(0x499cfa6a);
  pub const NUMBER: RuleId = RuleId(0x1bd670a0);
  pub const OCTAL_LITERAL: RuleId = RuleId(0x853e83d2);
  pub const OP_ASSIGN: RuleId = RuleId(0xf528b38c);
  pub const OP_ASSIGN_OPERATOR: RuleId = RuleId(0xe895ccd3);
  pub const OPTION_MAP: RuleId = RuleId(0x1683736b);
  pub const OPTION_MAPPING: RuleId = RuleId(0x76ce32df);
  pub const OPTION_VALUE: RuleId = RuleId(0x0baa2318);
  pub const OR: RuleId = RuleId(0x5d342984);
  pub const ORDERED_LIST: RuleId = RuleId(0xf8e6880d);
  pub const ORDERED_LIST_ITEM: RuleId = RuleId(0x3bef3dcf);
  pub const OUTPUT_OPERATOR: RuleId = RuleId(0x5c28bc8b);
  pub const OUTPUT_OPERATOR_A: RuleId = RuleId(0x34e84b49);
  pub const OUTPUT_OPERATOR_U: RuleId = RuleId(0x28e83865);
  pub const PARAGRAPH: RuleId = RuleId(0x8ffa6139);
  pub const PARAGRAPH_ELEMENT: RuleId = RuleId(0x72aba8e4);
  pub const PARAGRAPH_NEWLINE: RuleId = RuleId(0xaa368fb8);
  pub const PARAGRAPH_TEXT: RuleId = RuleId(0x6195191b);
  pub const PARENTHETICAL_TERM: RuleId = RuleId(0xe4e59e64);
  pub const PARSE: RuleId = RuleId(0x423b42ec);
  pub const PARSE_GRAMMAR: RuleId = RuleId(0x21b8fb74);
  pub const PARSE_MECH: RuleId = RuleId(0xaf070dc6);
  pub const PARSE_REPL_COMMAND: RuleId = RuleId(0xf69fc5f6);
  pub const PATTERN: RuleId = RuleId(0x873d0129);
  pub const PATTERN_ARRAY: RuleId = RuleId(0x39a90801);
  pub const PATTERN_ARRAY_ITEM: RuleId = RuleId(0x06d8e89b);
  pub const PATTERN_ARRAY_TOKEN: RuleId = RuleId(0xc551b941);
  pub const PATTERN_ATOM_STRUCT: RuleId = RuleId(0xe2a98d89);
  pub const PATTERN_TUPLE: RuleId = RuleId(0xd3fe175a);
  pub const PATTERN_TUPLE_STRUCT: RuleId = RuleId(0xcd363350);
  pub const PERCENT: RuleId = RuleId(0x75f9fa5a);
  pub const PERIOD: RuleId = RuleId(0x99c94704);
  pub const PLAN_RPL: RuleId = RuleId(0x178922b1);
  pub const PLUS: RuleId = RuleId(0xc4adc675);
  pub const POWER: RuleId = RuleId(0xf54f2346);
  pub const POWER_OPERATOR: RuleId = RuleId(0x9c9c7659);
  pub const PREFIXED_CONTEXT_PATH: RuleId = RuleId(0xb9c9a952);
  pub const PROFILE_RPL: RuleId = RuleId(0xb992f1cd);
  pub const PROGRAM: RuleId = RuleId(0x3d8466cb);
  pub const PROMPT: RuleId = RuleId(0xdfe6493b);
  pub const PROMPT_SIGIL: RuleId = RuleId(0xd534900e);
  pub const PROPER_SUBSET: RuleId = RuleId(0x60cb584c);
  pub const PROPER_SUPERSET: RuleId = RuleId(0x1224ecfb);
  pub const PUNCTUATION: RuleId = RuleId(0xbe3ef1b7);
  pub const QUESTION: RuleId = RuleId(0x06d5a845);
  pub const QUESTION_BLOCK: RuleId = RuleId(0x0afa1bc9);
  pub const QUESTION_SIGIL: RuleId = RuleId(0xf6b0a150);
  pub const QUIT_RPL: RuleId = RuleId(0x02b465b5);
  pub const QUOTE: RuleId = RuleId(0xb2887bd7);
  pub const QUOTE_BLOCK: RuleId = RuleId(0x851a4eeb);
  pub const QUOTE_SIGIL: RuleId = RuleId(0x71eb3c62);
  pub const RANGE_EXCLUSIVE: RuleId = RuleId(0xba78cbe3);
  pub const RANGE_EXPRESSION: RuleId = RuleId(0xc4fce9b3);
  pub const RANGE_INCLUSIVE: RuleId = RuleId(0x97191a09);
  pub const RANGE_OPERATOR: RuleId = RuleId(0xca143fc5);
  pub const RANGE_SUBSCRIPT: RuleId = RuleId(0xa81786c2);
  pub const RATIONAL_LITERAL: RuleId = RuleId(0x9ed5a559);
  pub const RAW_HYPERLINK: RuleId = RuleId(0x6b43470e);
  pub const RAW_STRING: RuleId = RuleId(0x82e2f075);
  pub const RAW_SUBTRACT: RuleId = RuleId(0x6f9c5a48);
  pub const RAW_TEXT: RuleId = RuleId(0xb5808b43);
  pub const REAL_NUMBER: RuleId = RuleId(0xb7a5a861);
  pub const RECORD: RuleId = RuleId(0x593058cc);
  pub const REFERENCE: RuleId = RuleId(0x5a81f39a);
  pub const REGULAR_TABLE: RuleId = RuleId(0x6d98dd22);
  pub const RELATIVE_SOURCE_IMPORT_SPECIFIER: RuleId = RuleId(0x48147430);
  pub const REPL_IDENTIFIER: RuleId = RuleId(0xec2868a0);
  pub const RIGHT_ALIGNMENT: RuleId = RuleId(0x992d85ad);
  pub const RIGHT_ANGLE: RuleId = RuleId(0x9eaa795b);
  pub const RIGHT_ANGLE1: RuleId = RuleId(0x305d21de);
  pub const RIGHT_ANGLE2: RuleId = RuleId(0x2f5d204b);
  pub const RIGHT_BRACE: RuleId = RuleId(0xc31f2933);
  pub const RIGHT_BRACKET: RuleId = RuleId(0xacb1a1b4);
  pub const RIGHT_JOIN: RuleId = RuleId(0xe40683c8);
  pub const RIGHT_PARENTHESIS: RuleId = RuleId(0xfcbd484c);
  pub const ROW_SEPARATOR: RuleId = RuleId(0x767c3ee5);
  pub const SAVE_RPL: RuleId = RuleId(0x869d77cb);
  pub const SCIENTIFIC_LITERAL: RuleId = RuleId(0xd8d7e236);
  pub const SECTION: RuleId = RuleId(0xfcdd0ccc);
  pub const SECTION_ELEMENT: RuleId = RuleId(0x520f1845);
  pub const SECTION_REFERENCE: RuleId = RuleId(0x5f8853d8);
  pub const SECTION_SIGIL: RuleId = RuleId(0x30175a21);
  pub const SELECT_ALL: RuleId = RuleId(0x3eda43b3);
  pub const SEMICOLON: RuleId = RuleId(0x0415d01e);
  pub const SEND_OPERATOR: RuleId = RuleId(0xe17e27f6);
  pub const SET: RuleId = RuleId(0xc6270703);
  pub const SET_COMPREHENSION: RuleId = RuleId(0xb27d4a02);
  pub const SET_OPERATOR: RuleId = RuleId(0x2f3b5e5a);
  pub const SLASH: RuleId = RuleId(0xcb73e8ea);
  pub const SLICE: RuleId = RuleId(0x6789b051);
  pub const SLICE_REF: RuleId = RuleId(0x6076ee69);
  pub const SOURCE_IMPORT_SPECIFIER: RuleId = RuleId(0x23a2f69b);
  pub const SOURCE_IMPORT_TAIL: RuleId = RuleId(0xcec6e017);
  pub const SOURCE_IMPORT_URI_SCHEME: RuleId = RuleId(0xd05ccd33);
  pub const SOURCE_MEC_PATH: RuleId = RuleId(0x0eaeb222);
  pub const SOURCE_MEC_PATH_WILDCARD_SUFFIX: RuleId = RuleId(0x5d51d6bb);
  pub const SOURCE_PATH_COMPONENT: RuleId = RuleId(0x096d5b3a);
  pub const SOURCE_PATH_COMPONENT_TOKEN: RuleId = RuleId(0xe9cba9a2);
  pub const SPACE: RuleId = RuleId(0x3553e285);
  pub const SPACE_TAB: RuleId = RuleId(0xddfa1f07);
  pub const SPACE_TAB0: RuleId = RuleId(0xa7bf2395);
  pub const SPACE_TAB1: RuleId = RuleId(0xa6bf2202);
  pub const SPACED_SUBTRACT: RuleId = RuleId(0x9008603e);
  pub const SPREAD_OPERATOR: RuleId = RuleId(0xbd1421c1);
  pub const SPREAD_OPERATOR_A: RuleId = RuleId(0xd244fa7f);
  pub const SPREAD_OPERATOR_U: RuleId = RuleId(0xde450d63);
  pub const STATEMENT: RuleId = RuleId(0xfb8d6d16);
  pub const STATEMENT_SEPARATOR: RuleId = RuleId(0x6d3cbe08);
  pub const STEP_RPL: RuleId = RuleId(0x67b402b8);
  pub const STRICT_EQUAL: RuleId = RuleId(0x55633ca1);
  pub const STRICT_NOT_EQUAL: RuleId = RuleId(0x9e40ed09);
  pub const STRIKE_SIGIL: RuleId = RuleId(0x9f47a3da);
  pub const STRIKETHROUGH: RuleId = RuleId(0x1eb28c44);
  pub const STRING: RuleId = RuleId(0x17c16538);
  pub const STRONG: RuleId = RuleId(0xc51f5d7a);
  pub const STRONG_SIGIL: RuleId = RuleId(0x52eb6537);
  pub const STRUCTURE: RuleId = RuleId(0x6f5525c6);
  pub const SUB_ASSIGN_OPERATOR: RuleId = RuleId(0x8d7f686c);
  pub const SUBLIST: RuleId = RuleId(0x8b09b311);
  pub const SUBSCRIPT: RuleId = RuleId(0xb27bbc7a);
  pub const SUBSET: RuleId = RuleId(0x4dab1c73);
  pub const SUBTITLE: RuleId = RuleId(0x115bfcb9);
  pub const SUBTRACT: RuleId = RuleId(0x42c1a561);
  pub const SUCCESS_BLOCK: RuleId = RuleId(0xfd7fb978);
  pub const SUCCESS_CHECK_SIGIL: RuleId = RuleId(0xc9b89aac);
  pub const SUCCESS_SIGIL: RuleId = RuleId(0x2ea879dd);
  pub const SUPERSET: RuleId = RuleId(0x0022ac1c);
  pub const SWIZZLE_SUBSCRIPT: RuleId = RuleId(0xf2757b45);
  pub const SYMBOL: RuleId = RuleId(0xf3fb51d1);
  pub const SYMBOLS_RPL: RuleId = RuleId(0x96497345);
  pub const SYMMETRIC_DIFFERENCE: RuleId = RuleId(0x448cc168);
  pub const SYNTH_OPERATOR: RuleId = RuleId(0x5b70298c);
  pub const TAB: RuleId = RuleId(0x98f72e4c);
  pub const TABLE: RuleId = RuleId(0x4a9c9bdf);
  pub const TABLE_COLUMN: RuleId = RuleId(0xf07e4b8c);
  pub const TABLE_END: RuleId = RuleId(0x683547b3);
  pub const TABLE_HEADER: RuleId = RuleId(0xff420217);
  pub const TABLE_HORZ: RuleId = RuleId(0x085c40c7);
  pub const TABLE_OPERATOR: RuleId = RuleId(0x1e434d06);
  pub const TABLE_ROW: RuleId = RuleId(0x283dfcaa);
  pub const TABLE_ROW2: RuleId = RuleId(0xf194a348);
  pub const TABLE_SEPARATOR: RuleId = RuleId(0xcb5b4979);
  pub const TABLE_START: RuleId = RuleId(0xa8301216);
  pub const TABLE_TOP: RuleId = RuleId(0x4e2be981);
  pub const TAG: RuleId = RuleId(0x95f72993);
  pub const TEXT: RuleId = RuleId(0xbde64e3e);
  pub const THEMATIC_BREAK: RuleId = RuleId(0xaa6ec998);
  pub const THIN_SPACE: RuleId = RuleId(0x9f3729bd);
  pub const TILDE: RuleId = RuleId(0xff1023a9);
  pub const TILDE_CODEBLOCK_SIGIL: RuleId = RuleId(0xa03569e3);
  pub const TITLE: RuleId = RuleId(0x9865b509);
  pub const TITLE_FRONT_MATTER: RuleId = RuleId(0xc0534f8b);
  pub const TRANSITION_OPERATOR: RuleId = RuleId(0x6dc19135);
  pub const TRANSITION_OPERATOR_A: RuleId = RuleId(0xd6b9c20b);
  pub const TRANSITION_OPERATOR_U: RuleId = RuleId(0xeab9e187);
  pub const TRANSPOSE: RuleId = RuleId(0xc03183c0);
  pub const TRUE_LITERAL: RuleId = RuleId(0x2a7b8757);
  pub const TUPLE: RuleId = RuleId(0x92722331);
  pub const TUPLE_DESTRUCTURE: RuleId = RuleId(0x6e0c5c64);
  pub const TUPLE_STRUCT: RuleId = RuleId(0xdd5df0e5);
  pub const TYPED_INTEGER: RuleId = RuleId(0x463d56ac);
  pub const UL_SUBTITLE: RuleId = RuleId(0x2d8e6d27);
  pub const UNCHECKED_ITEM: RuleId = RuleId(0xa4ce1a79);
  pub const UNDERLINE: RuleId = RuleId(0xe582347f);
  pub const UNDERLINE_SIGIL: RuleId = RuleId(0x4f35466a);
  pub const UNDERSCORE: RuleId = RuleId(0x34fdbabd);
  pub const UNDERSCORE_DIGIT: RuleId = RuleId(0xe34f0295);
  pub const UNION_OP: RuleId = RuleId(0x5655b904);
  pub const UNORDERED_LIST: RuleId = RuleId(0xb41d2946);
  pub const UNORDERED_LIST_ITEM: RuleId = RuleId(0xf5586d6a);
  pub const UNTYPED_INTEGER: RuleId = RuleId(0x119a93f5);
  pub const UNTYPED_REAL_NUMBER: RuleId = RuleId(0xc01271f1);
  pub const URI_SCHEME_PART: RuleId = RuleId(0x085c8a27);
  pub const URI_SOURCE_IMPORT_SPECIFIER: RuleId = RuleId(0x3c8a2e90);
  pub const UTF8_STRING: RuleId = RuleId(0x04a99a40);
  pub const VAR: RuleId = RuleId(0x8a25e7be);
  pub const VARIABLE_ASSIGN: RuleId = RuleId(0x562b9e49);
  pub const VARIABLE_DEFINE: RuleId = RuleId(0x47193f33);
  pub const WARNING_BLOCK: RuleId = RuleId(0x6ffb83a3);
  pub const WARNING_SIGIL: RuleId = RuleId(0xba3ac17a);
  pub const WHITESPACE: RuleId = RuleId(0xd240f720);
  pub const WHITESPACE0: RuleId = RuleId(0x0c44ee30);
  pub const WHITESPACE1: RuleId = RuleId(0x0d44efc3);
  pub const WHOS_RPL: RuleId = RuleId(0x71a8c9b7);
  pub const WILDCARD: RuleId = RuleId(0xb54fab6f);
  pub const WS0E: RuleId = RuleId(0x02f8cfc8);
  pub const WS1E: RuleId = RuleId(0xe6f6651d);
  pub const XOR: RuleId = RuleId(0xcc6bdb7e);
}
