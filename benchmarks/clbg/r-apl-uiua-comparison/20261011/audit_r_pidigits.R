# Work-count fixture; program AST unchanged except an appended diagnostic return.
suppressPackageStartupMessages(library(gmp))
root <- commandArgs(TRUE)[1L]
for (variant in c("compact","optimized")) {
  code <- parse(file.path(root,"sources","r","pidigits",paste0(variant,".R")))
  assignment <- Filter(function(x) is.call(x) && identical(x[[1L]],as.name("<-")) && identical(x[[2L]],as.name("run")),as.list(code))[[1L]]
  local <- new.env()
  local$divisions <- 0L
  local$tested <- 0L
  local$`%/%` <- function(x,y) {
    local$divisions <- local$divisions+max(length(x),length(y))
    base::`%/%`(x,y)
  }
  local$`!=` <- function(x,y) {
    local$tested <- local$tested+1L
    base::`!=`(x,y)
  }
  eval(assignment,local)
  original <- body(local$run)
  body(local$run) <- as.call(c(as.list(original),list(quote(invisible(k)))))
  output <- capture.output(iterations <- local$run(37L))
  stopifnot(local$divisions>=2L*local$tested)
  if (variant=="compact") stopifnot(local$divisions==2L*iterations)
  if (variant=="optimized") stopifnot(local$divisions==2L*local$tested)
  cat(sprintf("quotients\t%s\t37\t%d\t%d\t%d\n",variant,iterations,local$tested,local$divisions))
}
