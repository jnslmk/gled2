from functools import reduce

from stupidArtnet import StupidArtnet
from dataclasses import dataclass, fields
import struct

@dataclass
class ArtnetSceneControlState:
    scene_index: int = 0
    group_index: int = 0
    opacity: int = 0
    offset: int = 0
    speed_multiplier: int = 127
    color_mode: int = 0
    pallet_override: int = 0

    def __post_init__(self):
        for f in fields(self):
            v = getattr(self, f.name)
            if not (0 <= v <= 255):
                raise ValueError(f"{f.name} must be 0-255")

    def __bytes__(self):
        # AUTO-generates 'BBBBBBB' from field count
        fmt = f'{len(fields(self))}B'
        return struct.pack(fmt, *[getattr(self, f.name) for f in fields(self)])

if __name__ == "__main__":
    # THESE ARE MOST LIKELY THE VALUES YOU WILL BE NEEDING
    target_ip = '127.0.0.1'		# typically in 2.x or 10.x range
    universe = 1337
    packet_size = 100								# it is not necessary to send whole universe
    # CREATING A STUPID ARTNET OBJECT
    # SETUP NEEDS A FEW ELEMENTS
    # TARGET_IP   = DEFAULT 127.0.0.1
    # UNIVERSE    = DEFAULT 0
    # PACKET_SIZE = DEFAULT 512
    # FRAME_RATE  = DEFAULT 30
    # ISBROADCAST = DEFAULT FALSE

    # By default, the server uses port 6454, no need to specify it.
    # If you need to change the Art-Net port, ensure the port is within the valid range for UDP ports (1024-65535).
    # Be sure that no other application is using the selected port on your network.
    # To specify a different port, for example port 6455, you can do it like this:
    # a = StupidArtnetServer(target_ip, universe, packet_size, 30, True, True, port=6455 )  # Change 6455 to any valid port number between 1024 and 65535.

    a = StupidArtnet( target_ip, universe, packet_size, 30, True, True )

    # MORE ADVANCED CAN BE SET WITH SETTERS IF NEEDED
    # NET         = DEFAULT 0
    # SUBNET      = DEFAULT 0

    # CHECK INIT
    print(a)

    states = [ArtnetSceneControlState() for i in range(10)]

    states[0] = ArtnetSceneControlState(scene_index=1, opacity=128)
    states[1] = ArtnetSceneControlState(scene_index=0)
    states[3] = ArtnetSceneControlState(scene_index=3, offset=128, speed_multiplier=129)

    packet = reduce(lambda x, y: x + bytes(y), states, bytearray())
    packet.extend(b'\x00' * (packet_size - len(packet)))
    print(packet)

    a.set(packet)
    a.show()