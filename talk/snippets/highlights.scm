; MJML root/structure tags — highlighted as keywords
((tag_name) @keyword
  (#match? @keyword "^(mjml|mj-head|mj-body|mj-include)$"))

; Fallback: all other tag names
(tag_name) @tag
