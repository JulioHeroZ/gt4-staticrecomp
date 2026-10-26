#include "dfp_rpc.hpp"

#include <algorithm>
#include <cstring>

namespace gt4::dfp {

namespace {

void write_u32_le(std::span<std::uint8_t> buffer, std::size_t offset, std::uint32_t value) noexcept {
  if (offset + 4u <= buffer.size()) {
    buffer[offset + 0] = static_cast<std::uint8_t>(value & 0xFFu);
    buffer[offset + 1] = static_cast<std::uint8_t>((value >> 8) & 0xFFu);
    buffer[offset + 2] = static_cast<std::uint8_t>((value >> 16) & 0xFFu);
    buffer[offset + 3] = static_cast<std::uint8_t>((value >> 24) & 0xFFu);
  }
}

std::uint32_t read_u32_le(std::span<const std::uint8_t> buffer, std::size_t offset) noexcept {
  if (offset + 4u <= buffer.size()) {
    return static_cast<std::uint32_t>(buffer[offset + 0]) |
           (static_cast<std::uint32_t>(buffer[offset + 1]) << 8) |
           (static_cast<std::uint32_t>(buffer[offset + 2]) << 16) |
           (static_cast<std::uint32_t>(buffer[offset + 3]) << 24);
  }
  return 0u;
}

} // namespace

void DfpRpcServer::update_input(const DfpInputState &state) noexcept {
  input_ = state;
}

DfpInputState DfpRpcServer::input_state() const noexcept {
  return input_;
}

bool DfpRpcServer::poll_ffb(DfpFfbPacket &out_packet) noexcept {
  if (ffb_count_ == 0)
    return false;

  out_packet = ffb_queue_[ffb_head_];
  ffb_head_ = (ffb_head_ + 1u) % kFfbQueueCapacity;
  --ffb_count_;
  return true;
}

void DfpRpcServer::push_ffb(const std::uint8_t *payload, std::size_t length) noexcept {
  if (length == 0 || payload == nullptr)
    return;

  DfpFfbPacket packet{};
  const std::size_t copy_bytes = std::min(length, sizeof(packet.data));
  std::memcpy(packet.data, payload, copy_bytes);

  if (ffb_count_ < kFfbQueueCapacity) {
    ffb_queue_[ffb_tail_] = packet;
    ffb_tail_ = (ffb_tail_ + 1u) % kFfbQueueCapacity;
    ++ffb_count_;
  } else {
    // Queue overflow: overwrite oldest packet to preserve freshest physics command
    ffb_queue_[ffb_head_] = packet;
    ffb_head_ = (ffb_head_ + 1u) % kFfbQueueCapacity;
  }
}

void DfpRpcServer::reset() noexcept {
  input_ = {};
  stats_ = {};
  ffb_head_ = 0;
  ffb_tail_ = 0;
  ffb_count_ = 0;
}

std::size_t DfpRpcServer::dispatch(std::uint32_t rpc_number,
                                   std::span<const std::uint8_t> request,
                                   std::span<std::uint8_t> reply) noexcept {
  if (!enabled_)
    return 0u;

  ++stats_.total_calls;

  // Clear reply buffer initially to ensure deterministic memory state
  if (!reply.empty()) {
    std::fill(reply.begin(), reply.end(), static_cast<std::uint8_t>(0));
  }

  const auto fn = static_cast<RpcFunction>(rpc_number);
  switch (fn) {
  case RpcFunction::DeviceSync: {
    // RPC 12: Handshake sync. Checked at EE address 0x005608BC.
    // Word at +0x04 must be 0x010B2400 (completed).
    ++stats_.sync_calls;
    if (reply.size() >= 8u) {
      write_u32_le(reply, 4u, kSyncStatusCompleted);
    }
    return std::min(reply.size(), kRpcBufferSize);
  }

  case RpcFunction::QueryIdentity: {
    // RPC 4: Query identity and media signature. Checked at EE 0x0055585C.
    // Word at +0x5C must be 0x046DC298 (Logitech Driving Force Pro).
    ++stats_.identity_calls;
    if (reply.size() >= 0x60u) {
      write_u32_le(reply, 0x5Cu, kMediaSignatureDfp);
    }
    return std::min(reply.size(), kRpcBufferSize);
  }

  case RpcFunction::ArmReadSlots: {
    // RPC 5: Arm read slots for asynchronous polling.
    ++stats_.arm_calls;
    if (reply.size() >= 8u) {
      write_u32_le(reply, 4u, 1u); // Acknowledged
    }
    return std::min(reply.size(), kRpcBufferSize);
  }

  case RpcFunction::InputPoll: {
    // RPC 6: Input polling report exchange.
    ++stats_.poll_calls;
    if (reply.size() >= 32u) {
      // Offset +0x00: Status flags (1 = connected & active)
      write_u32_le(reply, 0x00u, input_.connected ? 1u : 0u);

      // Offset +0x04: 14-bit steering angle (0..16383, centered at 8192)
      const std::uint32_t steering_clamped = std::min<std::uint32_t>(input_.steering, kSteeringMax);
      write_u32_le(reply, 0x04u, steering_clamped);

      // Offset +0x08: Throttle pressure (0..255)
      write_u32_le(reply, 0x08u, static_cast<std::uint32_t>(input_.throttle));

      // Offset +0x0C: Brake pressure (0..255)
      write_u32_le(reply, 0x0Cu, static_cast<std::uint32_t>(input_.brake));

      // Offset +0x10: Button bitmask
      write_u32_le(reply, 0x10u, static_cast<std::uint32_t>(input_.buttons));

      // Offset +0x14: D-pad hat status
      write_u32_le(reply, 0x14u, static_cast<std::uint32_t>(input_.dpad));

      // Offset +0x18: Wheel mode flags (bit 0 = 900-degree mode active)
      write_u32_le(reply, 0x18u, mode_900_degrees_ ? 1u : 0u);
    }
    return std::min(reply.size(), kRpcBufferSize);
  }

  case RpcFunction::FfbCommand: {
    // RPC 13 (0x0D): Force feedback command block from GT4 physics engine.
    ++stats_.ffb_calls;
    if (!request.empty()) {
      push_ffb(request.data(), request.size());
    }
    if (reply.size() >= 8u) {
      write_u32_le(reply, 4u, 1u); // Command accepted
    }
    return std::min<std::size_t>(reply.size(), 16u);
  }

  case RpcFunction::ModeConfig: {
    // RPC 15 (0x0F): Mode and rotation angle configuration (900-degree unlock).
    ++stats_.mode_calls;
    if (request.size() >= 8u) {
      const std::uint32_t requested_mode = read_u32_le(request, 4u);
      if (requested_mode != 0) {
        mode_900_degrees_ = true;
      }
    }
    if (reply.size() >= 8u) {
      write_u32_le(reply, 4u, mode_900_degrees_ ? 0x0384u : 0x00C8u); // 900 vs 200 degrees
    }
    return std::min(reply.size(), kRpcBufferSize);
  }

  default:
    ++stats_.unknown_calls;
    return 0u;
  }
}

} // namespace gt4::dfp
