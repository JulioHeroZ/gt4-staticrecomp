#pragma once

#include <cstddef>
#include <cstdint>
#include <span>
#include <string_view>

namespace gt4::dfp {

// Logitech Device SIF RPC constants
inline constexpr std::uint32_t kLogitechServerId = 0x046D046Du;
inline constexpr std::size_t kRpcBufferSize = 576u;

// Status codes and signatures
inline constexpr std::uint32_t kSyncStatusCompleted = 0x010B2400u;
inline constexpr std::uint32_t kMediaSignatureDfp = 0x046DC298u; // VID: 0x046D (Logitech), PID: 0xC298 (DFP)
inline constexpr std::string_view kDriverBanner = "liblgdev version 1.11.036";

// Known RPC Function numbers on server 0x046D046D
enum class RpcFunction : std::uint32_t {
  QueryIdentity = 4u,
  ArmReadSlots = 5u,
  InputPoll = 6u,
  DeviceSync = 12u,
  FfbCommand = 13u, // 0x0D
  ModeConfig = 15u, // 0x0F
  Unknown = 0xFFFFFFFFu,
};

// 14-bit steering angle (0 to 16383), center is 8192
inline constexpr std::uint16_t kSteeringCenter = 8192u;
inline constexpr std::uint16_t kSteeringMax = 16383u;

// Guest DFP Input Report State
struct DfpInputState {
  std::uint16_t steering = kSteeringCenter; // 14-bit angle (0..16383)
  std::uint8_t throttle = 0;               // 0..255
  std::uint8_t brake = 0;                  // 0..255
  std::uint16_t buttons = 0;               // Digital buttons mask
  std::uint8_t dpad = 0;                   // Directional pad hat
  bool connected = true;                   // Virtual device link status
};

// 8-byte Force Feedback Command Packet from GT4 physics engine
struct DfpFfbPacket {
  std::uint8_t data[8]{};

  [[nodiscard]] std::uint8_t command() const noexcept { return data[0]; }
};

// Telemetry and statistics for diagnostic reporting
struct DfpRpcStats {
  std::uint64_t total_calls = 0;
  std::uint64_t sync_calls = 0;
  std::uint64_t identity_calls = 0;
  std::uint64_t arm_calls = 0;
  std::uint64_t poll_calls = 0;
  std::uint64_t ffb_calls = 0;
  std::uint64_t mode_calls = 0;
  std::uint64_t unknown_calls = 0;
};

// High-Level Emulation (HLE) Provider for Logitech SIF RPC Server 0x046D046D
class DfpRpcServer {
public:
  DfpRpcServer() noexcept = default;

  // Configuration
  void set_enabled(bool enabled) noexcept { enabled_ = enabled; }
  [[nodiscard]] bool is_enabled() const noexcept { return enabled_; }

  void set_900_degrees(bool enable_900) noexcept { mode_900_degrees_ = enable_900; }
  [[nodiscard]] bool is_900_degrees() const noexcept { return mode_900_degrees_; }

  // Input state feeder (called by host SDL3 bridge)
  void update_input(const DfpInputState &state) noexcept;
  [[nodiscard]] DfpInputState input_state() const noexcept;

  // Force feedback queue inspection (called by host SDL3 haptics dispatcher)
  [[nodiscard]] bool poll_ffb(DfpFfbPacket &out_packet) noexcept;

  // SIF RPC Dispatcher: processes incoming request and writes reply
  // Returns number of bytes written to reply buffer, or 0 on unhandled call
  std::size_t dispatch(std::uint32_t rpc_number,
                       std::span<const std::uint8_t> request,
                       std::span<std::uint8_t> reply) noexcept;

  // Diagnostics
  [[nodiscard]] const DfpRpcStats &stats() const noexcept { return stats_; }
  void reset_stats() noexcept { stats_ = {}; }

  // Reset internal state
  void reset() noexcept;

private:
  bool enabled_ = true;
  bool mode_900_degrees_ = true;
  DfpInputState input_{};
  DfpRpcStats stats_{};

  // Ring buffer for pending FFB packets from guest to host
  static constexpr std::size_t kFfbQueueCapacity = 32u;
  DfpFfbPacket ffb_queue_[kFfbQueueCapacity]{};
  std::size_t ffb_head_ = 0;
  std::size_t ffb_tail_ = 0;
  std::size_t ffb_count_ = 0;

  void push_ffb(const std::uint8_t *payload, std::size_t length) noexcept;
};

} // namespace gt4::dfp
