/// Revert with this value for a failing call to `require`
pub const FAILED_REQUIRE_SIGNAL: u64 = 0xffff_ffff_ffff_0000;

/// Revert with this value for a failing call to `transfer_to_address`.
pub const FAILED_TRANSFER_TO_ADDRESS_SIGNAL: u64 = 0xffff_ffff_ffff_0001;

/// Revert with this value for a failing call to `send_message`.
pub const FAILED_SEND_MESSAGE_SIGNAL: u64 = 0xffff_ffff_ffff_0002;

/// Revert with this value for a failing call to `assert_eq`.
pub const FAILED_ASSERT_EQ_SIGNAL: u64 = 0xffff_ffff_ffff_0003;

/// Revert with this value for a failing call to `assert`.
pub const FAILED_ASSERT_SIGNAL: u64 = 0xffff_ffff_ffff_0004;
