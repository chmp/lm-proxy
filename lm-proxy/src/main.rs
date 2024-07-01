mod lm_proxy;

fn main() -> anyhow::Result<()> {
    argh::from_env::<lm_proxy::cmd::Args>().run()
}
