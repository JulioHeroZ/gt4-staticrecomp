#include "dfp_rpc.hpp"
#include "dfp_input.hpp"

#include <array>
#include <cstdint>
#include <cstdio>
#include <stdexcept>
#include <string>

namespace {

void require(bool condition, const char *label) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", label);
    throw std::runtime_error(label);
  }
}

std::uint32_t get_u32_le(const std::uint8_t *buffer, std::size_t offset) {
  return static_cast<std::uint32_t>(buffer[offset + 0]) |
         (static_cast<std::uint32_t>(buffer[offset + 1]) << 8) |
         (static_cast<std::uint32_t>(buffer[offset + 2]) << 16) |
         (static_cast<std::uint32_t>(buffer[offset + 3]) << 24);
}

void test_sync_handshake() {
  gt4::dfp::DfpRpcServer server;
  std::array<std::uint8_t, 576> request{};
  std::array<std::uint8_t, 576> reply{};

  const std::size_t written = server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::DeviceSync),
                                              request, reply);
  require(written == 576, "Sync RPC must return full 576 byte buffer");
  const std::uint32_t status = get_u32_le(reply.data(), 4u);
  require(status == gt4::dfp::kSyncStatusCompleted, "Status word at +4 must be 0x010B2400 (completed)");
  require(server.stats().sync_calls == 1, "Sync calls stat incremented");
}

void test_identity_handshake() {
  gt4::dfp::DfpRpcServer server;
  std::array<std::uint8_t, 576> request{};
  std::array<std::uint8_t, 576> reply{};

  const std::size_t written = server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::QueryIdentity),
                                              request, reply);
  require(written == 576, "Identity RPC must return full 576 byte buffer");
  const std::uint32_t signature = get_u32_le(reply.data(), 0x5Cu);
  require(signature == gt4::dfp::kMediaSignatureDfp, "Media signature at +0x5C must be 0x046DC298 (Logitech DFP)");
  require(server.stats().identity_calls == 1, "Identity calls stat incremented");
}

void test_negative_control() {
  gt4::dfp::DfpRpcServer server;
  server.set_enabled(false);

  std::array<std::uint8_t, 576> request{};
  std::array<std::uint8_t, 576> reply{};

  const std::size_t written_sync = server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::DeviceSync),
                                                   request, reply);
  require(written_sync == 0, "Disabled server must ignore DeviceSync RPC");

  const std::size_t written_id = server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::QueryIdentity),
                                                 request, reply);
  require(written_id == 0, "Disabled server must ignore QueryIdentity RPC");
  require(server.stats().total_calls == 0, "No calls logged when disabled");
}

void test_mode_and_range() {
  gt4::dfp::DfpRpcServer server;
  std::array<std::uint8_t, 576> request{};
  std::array<std::uint8_t, 576> reply{};

  // Set request payload requesting 900-degree mode (flag 1 at offset 4)
  request[4] = 1;

  const std::size_t written = server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::ModeConfig),
                                              request, reply);
  require(written == 576, "ModeConfig RPC must write reply buffer");
  require(server.is_900_degrees(), "Server must be in 900 degree mode");
  const std::uint32_t reported_degrees = get_u32_le(reply.data(), 4u);
  require(reported_degrees == 0x0384u, "Reported degrees must be 900 (0x0384)");
}

void test_input_poll_report() {
  gt4::dfp::DfpRpcServer server;

  gt4::dfp::DfpInputState input{};
  input.steering = 12000u; // Right turn
  input.throttle = 220u;
  input.brake = 85u;
  input.buttons = 0x0001u | 0x0008u; // Cross + Square
  input.dpad = 2u;
  input.connected = true;
  server.update_input(input);

  std::array<std::uint8_t, 576> request{};
  std::array<std::uint8_t, 576> reply{};

  const std::size_t written = server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::InputPoll),
                                              request, reply);
  require(written == 576, "InputPoll RPC must write reply buffer");

  require(get_u32_le(reply.data(), 0x00u) == 1u, "Connected flag active");
  require(get_u32_le(reply.data(), 0x04u) == 12000u, "14-bit steering angle matched");
  require(get_u32_le(reply.data(), 0x08u) == 220u, "Throttle pressure matched");
  require(get_u32_le(reply.data(), 0x0Cu) == 85u, "Brake pressure matched");
  require(get_u32_le(reply.data(), 0x10u) == (0x0001u | 0x0008u), "Buttons matched");
  require(get_u32_le(reply.data(), 0x14u) == 2u, "Dpad matched");
  require(get_u32_le(reply.data(), 0x18u) == 1u, "900-degree mode bit active");
}

void test_ffb_packet_queue() {
  gt4::dfp::DfpRpcServer server;

  std::array<std::uint8_t, 8> ffb_cmd1{0x00, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}; // Constant force
  std::array<std::uint8_t, 8> ffb_cmd2{0x02, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00}; // Spring

  std::array<std::uint8_t, 16> reply{};

  server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::FfbCommand), ffb_cmd1, reply);
  server.dispatch(static_cast<std::uint32_t>(gt4::dfp::RpcFunction::FfbCommand), ffb_cmd2, reply);

  require(server.stats().ffb_calls == 2, "Two FFB calls registered");

  gt4::dfp::DfpFfbPacket out_pkt{};
  require(server.poll_ffb(out_pkt), "First FFB packet polled");
  require(out_pkt.command() == 0x00, "First packet command is constant force");
  require(out_pkt.data[1] == 0x7F, "First packet magnitude matched");

  require(server.poll_ffb(out_pkt), "Second FFB packet polled");
  require(out_pkt.command() == 0x02, "Second packet command is spring");

  require(!server.poll_ffb(out_pkt), "Queue must now be empty");
}

void test_input_bridge_encoding() {
  using namespace gt4::dfp;
  // Center
  const auto center = DfpInputBridge::encode_report(0.0f, 0.0f, 0.0f, 0, 0, true);
  require(center.steering == kSteeringCenter, "Center steering must be 8192");
  require(center.throttle == 0, "Zero throttle is 0");
  require(center.brake == 0, "Zero brake is 0");

  // Full right & full throttle
  const auto full_right = DfpInputBridge::encode_report(1.0f, 1.0f, 0.0f, 0x1234, 4, true);
  require(full_right.steering == kSteeringMax, "Full right steering is 16383");
  require(full_right.throttle == 255, "Full throttle is 255");
  require(full_right.brake == 0, "Brake is 0");
  require(full_right.buttons == 0x1234, "Buttons mask preserved");
  require(full_right.dpad == 4, "Dpad preserved");

  // Full left & full brake
  const auto full_left = DfpInputBridge::encode_report(-1.0f, 0.0f, 1.0f, 0, 0, true);
  require(full_left.steering == 0, "Full left steering is 0");
  require(full_left.throttle == 0, "Throttle is 0");
  require(full_left.brake == 255, "Full brake is 255");

  // Clamping outside bounds
  const auto clamped = DfpInputBridge::encode_report(2.5f, 3.0f, -1.0f, 0, 0, true);
  require(clamped.steering == kSteeringMax, "Clamped steering cannot exceed 16383");
  require(clamped.throttle == 255, "Clamped throttle cannot exceed 255");
  require(clamped.brake == 0, "Clamped brake cannot go below 0");

  // Degrees to 14-bit
  require(DfpInputBridge::degrees_to_raw_14bit(0.0f, 900.0f) == kSteeringCenter, "0 deg is center");
  require(DfpInputBridge::degrees_to_raw_14bit(450.0f, 900.0f) == kSteeringMax, "450 deg is max");
  require(DfpInputBridge::degrees_to_raw_14bit(-450.0f, 900.0f) == 0, "-450 deg is min");

  // Disconnect handling
  const auto disconnected = DfpInputBridge::encode_report(1.0f, 1.0f, 1.0f, 0xFFFF, 8, false);
  require(!disconnected.connected, "Connected is false");
  require(disconnected.steering == kSteeringCenter, "Disconnected centers steering for safety");
}

} // namespace

int main() {
  try {
    test_sync_handshake();
    test_identity_handshake();
    test_negative_control();
    test_mode_and_range();
    test_input_poll_report();
    test_ffb_packet_queue();
    test_input_bridge_encoding();
    std::printf("All DFP RPC and Input Bridge fixture tests PASSED (7/7).\n");
    return 0;
  } catch (const std::exception &ex) {
    std::fprintf(stderr, "DFP RPC fixture FAILED: %s\n", ex.what());
    return 1;
  }
}
