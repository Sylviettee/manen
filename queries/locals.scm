; Scopes

[
  (chunk)
  (do_statement)
  (while_statement)
  (repeat_statement)
  (if_statement)
  (for_statement)
  (function_definition)
  (function_declaration)
] @local.scope

; Definitions

(assignment_statement
  (variable_list
    (identifier) @local.definition))

; technically a "local function name() ... end" is
;   local name
;   name = function() ... end
; so we need to ensure that the function name goes into the parent scope
; to do what we treat it as a special case and hoist it in the code
(function_declaration
  name: (identifier) @local.fn_name)

(for_generic_clause
  (variable_list
    (identifier) @local.definition))

(for_numeric_clause
  name: (identifier) @local.definition)

(parameters (identifier) @local.definition)
