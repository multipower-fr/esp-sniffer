use interoptopus::util::NamespaceMappings;
use interoptopus::{Error, Interop};

#[test]
fn bindings_cpython_cffi() -> Result<(), Error> {
    use interoptopus_backend_cpython::{Config, Generator};

    let library = libwifisnipe_test::ffi_inventory();

    Generator::new(Config::default(), library)
        .write_file("bindings/python/libwifisnipe_test.py")?;

    Ok(())
}