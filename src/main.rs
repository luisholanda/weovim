mod nvim;

fn main() {
    let _neovim = nvim::start_neovim();

    loop {}
}
