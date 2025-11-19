open Unix

(* Logging helper *)
let log_info msg =
  let now = gettimeofday () in
  let tm = localtime now in
  Printf.fprintf stderr "[%04d-%02d-%02d %02d:%02d:%02d] INFO: %s\n%!"
    (1900 + tm.tm_year) (1 + tm.tm_mon) tm.tm_mday
    tm.tm_hour tm.tm_min tm.tm_sec
    msg

let () =
  log_info "Starting ebook renamer";
  
  (* Get command line arguments *)
  let args = Array.to_list Sys.argv in
  let path = if List.length args > 1 then List.nth args 1 else "." in
  
  log_info ("Processing path: " ^ path);
  
  (* For now, this is a minimal implementation showing the structure *)
  (* Full implementation would include:
     - CLI argument parsing using Cmdliner
     - File scanning with recursion
     - Filename normalization
     - Duplicate detection
     - Todo list generation *)
  
  print_endline "OCaml implementation - work in progress";
  print_endline "This is a placeholder showing the logging structure";
  print_endline "Full implementation requires:";
  print_endline "  - CLI parsing module";
  print_endline "  - Scanner module";
  print_endline "  - Normalizer module";
  print_endline "  - Duplicates module";
  print_endline "  - Todo module"
