from stupidArtnet import StupidArtnet

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

    # YOU CAN CREATE YOUR OWN BYTE ARRAY OF PACKET_SIZE
    packet = bytearray(packet_size)		# create packet for Artnet
    for i in range(packet_size):			# fill packet with sequential values
        packet[i] = (i % 256)

    # ... AND SET IT TO STUPID ARTNET
    a.set(packet)						# only on changes

    # ALL PACKETS ARE SAVED IN THE CLASS, YOU CAN CHANGE SINGLE VALUES
    a.set_single_value(1, 255)			# set channel 1 to 255

    # ... AND SEND
    a.show()							# send data

    # OR USE STUPIDARTNET FUNCTIONS
    #a.flash_all()						# send single packet with all channels at 255