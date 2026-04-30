use akave_rs::cli::run_from_args;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let (stdout, stderr, success) = run_from_args(&args_ref).await;
    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    if !success {
        std::process::exit(1);
    }
}
