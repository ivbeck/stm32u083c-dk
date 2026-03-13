#![no_std]
#![no_main]

use stm32u083c_dk as _; // memory layout + panic handler

struct TestState {}

#[defmt_test::tests]
mod tests {
    use defmt::assert_eq;

    #[init]
    fn init() -> super::TestState {
        super::TestState {}
    }

    // This function is called before each test case.
    // It accesses the state created in `init`,
    // though like with `test`, state access is optional.
    #[before_each]
    fn before_each() {
        defmt::println!("Starting test");
    }

    // This function is called after each test
    #[after_each]
    fn after_each() {
        defmt::println!("Done");
    }

    // this unit test doesn't access the state
    #[test]
    fn add_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
