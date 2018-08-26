#[macro_use]
extern crate oni_trace;

// Put this in a seperate module to check that the module expansion is working
mod profile_tests {
    use std::fs::File;
    use std::io::prelude::Read;
    use std::string::String;
    use std::thread::sleep;
    use std::time::Duration;
    use oni_trace;

    #[test]
    fn test_profile_macro() {
        let output_file = format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            "integration_test_macro_output.json"
        );
        oni_trace::register_thread_with_profiler();
        {
            oni_trace_scope![MyTestProfile];

            oni_async_event!(start Fuck [ab] => 5);

            {
                oni_trace_scope![Sleep 1];
                sleep(Duration::from_millis(50));
            }

            oni_async_event!(instant Fuck [ab] => 5);

            sleep(Duration::from_millis(20));
            oni_async_event!(instant Fuck [ab] => 5);

            oni_instant!("some event");

            sleep(Duration::from_millis(10));

            oni_async_event!(end Fuck [ab] => 5);

            {
                oni_trace_scope![Sleep 2];
                sleep(Duration::from_millis(50));
            }
        }
        oni_trace::write_profile_json(&output_file);

        // Get the profile that we wrote
        let mut f = File::open(output_file).unwrap();
        let mut buffer = String::new();
        f.read_to_string(&mut buffer).unwrap();

        // Test that the correct name has been used for the profile scope
        // but only if we have the feature enabled#
        let test = buffer.contains("MyTestProfile");
            //&& buffer.contains(r#""module":"profile_tests""#);

        if cfg!(feature = "oni_trace") {
            assert!(
                test,
                "Integration test macro did not contain the correct profile name",
            );
        } else {
            assert!(
                !test,
                "Integration test macro incorrectly contained the profile name"
            );
        }
    }
}
