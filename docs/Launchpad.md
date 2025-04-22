MIDI on Launchpad X
The Launchpad X has two MIDI interfaces providing two pairs of MIDI inputs and outputs over USB.
They are as follows:
- LPX DAW In / Out (or first interface on Windows): This interface is used by DAWs and similar
  software to interact with the Launchpad X’s Session mode.
- LPX MIDI In / Out (or second interface on Windows): This interface is used to receive MIDI
  from Note mode and Custom modes; and is used to provide external MIDI input or Light
  controls in Lighting Custom Modes and Programmer mode.
  If you wish to use Launchpad X as a control surface for a DAW (Digital Audio Workstation), you will
  likely want to use the DAW interface (See Software Interaction chapter).
  Otherwise, you may interact with the device using the MIDI interface.
  The Launchpad X sends Note On (90h – 9Fh) with velocity zero for Note Offs. It accepts either Note
  Offs (80h – 8Fh) or Note Ons (90h – 9Fh) with velocity zero for Note Off.
  Device Inquiry message
  The Launchpad X responds to the Universal Device Inquiry Sysex message, which can be used to
  identify the device. This exchange is as follows:
  Host => Launchpad X:
  Hex: F0h 7Eh 7Fh 06h 01h F7h
  Dec: 240 126 127 6 1 247
  Launchpad X => Host (Application):
  Hex: F0h 7Eh 00h 06h 02h 00h 20h 29h 13h 01h 00h 00h <app_version> F7h
  Dec: 240 126 0 6 2 0 32 41 19 1 0 0 <app_version> 247
  Launchpad X => Host (Bootloader):
  Hex: F0h 7Eh 00h 06h 02h 00h 20h 29h 13h 11h 00h 00h <boot_version> F7h
  Dec: 240 126 0 6 2 0 32 41 19 17 0 0 <boot_version> 247
  The <app_version> or <boot_version> field is 4 bytes long, providing the Application or the Bootloader
  version respectively. The version is the same version which can be viewed using the lower left green
  pads on the Bootloader’s surface, provided as four bytes, each byte corresponding to one digit,
  ranging from 0 – 9.
  SysEx message format used by the device
  All SysEx messages begin with the following header regardless of direction (Host => Launchpad X or
  Launchpad X => Host):
  Hex: F0h 00h 20h 29h 02h 0Ch
  Dec: 240 0 32 41 2 12
  After the header, a command byte follows, selecting the function to use.
  Several of the messages have a readback variant which can be accessed in the following manner:
  7
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch <command> F7h
  Dec: 240 0 32 41 2 12 <command> 247
  Launchpad X => Host:
  Hex: F0h 00h 20h 29h 02h 0Ch <command> <data> F7h
  Dec: 240 0 32 41 2 12 <command> <data> 247
  Where the <data> is formatted in the same manner as normally it would be provided to the Launchpad
  X after the command. These readback forms are described for each of the commands where available.
  Selecting layouts
  The Launchpad X has several layouts to choose from, which can be controlled by either the device’s
  User Interface (see the User Guide for more details), or the following SysEx message:
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch 00h <layout> F7h
  Dec: 240 0 32 41 2 12 0 <layout> 247
  Where the available layouts are:
- 00h (0): Session (only selectable in DAW mode)
- 01h (1): Note mode
- 04h (4): Custom mode 1 (Drum Rack by factory default)
- 05h (5): Custom mode 2 (Keys by factory default)
- 06h (6): Custom mode 3 (Lighting mode in Drum Rack layout by factory default)
- 07h (7): Custom mode 4 (Lighting mode in Session layout by factory default)
- 0Dh (13): DAW Faders (only selectable in DAW mode)
- 7Fh (127): Programmer mode
  Readback variant is available by the following SysEx message:
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch 00h F7h
  Dec: 240 0 32 41 2 12 0 247
  When selecting Programmer mode using this SysEx message, the Setup entry (holding down Session
  for half a second) is disabled. To return the Launchpad X to normal operation, use this SysEx message
  to select any other layout than Programmer mode.
  Programmer / Live mode switch
  There is a dedicated SysEx message for Programmer / Live mode select:
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch 0Eh <mode> F7h
  Dec: 240 0 32 41 2 12 14 <mode> 247
  Where <mode> is 0 for Live mode, 1 for Programmer mode.
  Readback variant is available by the following SysEx message:
  8
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch 0Eh F7h
  Dec: 240 0 32 41 2 12 14 247
  When selecting Live mode with this message, Launchpad X switches to Session layout, or Note mode
  when not in DAW mode.
  When selecting Programmer mode using this SysEx message, the Setup entry (holding down Session
  for half a second) is disabled. To return the Launchpad X to normal operation, use this SysEx message
  to switch back to Live mode.
  9
  Controlling the Surface
  This chapter describes the wide range of possibilities for controlling and lighting up the Launchpad X
  surface. If you aim to light Launchpad X’s surface, this chapter contains all the information you will
  likely need for it. You can also use these capabilities to realize surfaces for interacting with software,
  however for that purpose, using the DAW mode is preferable as it keeps the MIDI port free for playing
  and interacting with virtual instruments. See chapter Software Interaction.
  Switching to Lighting modes
  If you aim to control the Launchpad X using scripts, the best option for lighting pads/buttons or
  creating an interactive surface is switching to Programmer mode. You can use the Programmer / Live
  mode switch message to achieve this or access it from the setup menu (hold Session, then press
  bottom Scene Launch button). Programmer Mode sends out a message for every pad or button that
  is pressed, and equally lights pads or buttons when the same message is received Remember to switch
  the device back to Live mode once done.
  Alternatively, you may use Lighting Custom Modes in the same way, but you are limited to interacting
  only with the 8x8 grid. On a Launchpad X in its factory default state, Custom mode 3 and Custom mode
  4 are set up for lighting pads, each with a different layout. You can switch to either of these using the
  Launchpad X’s User Interface. In these Lighting Custom Modes, you may enable Ghost mode which
  allows the lighting area to expand beyond the 8x8 grid into the top and side button rows and logo.
  Access Ghost Mode by pressing the Note and Custom buttons in rapid succession (with the
  appropriate Custom mode already selected).
  In a Lighting Custom Mode, Ghost mode is automatically enabled when:
- Sending any Control Change message on the MIDI interface (these can be used to light up the
  top row of buttons, the logo and the right-side buttons).
- Sending the LED lighting SysEx message.
  10
  Programmer mode layout
  Note that in Programmer mode, all buttons and pads accept either Note or Control Change messages.
  The indicated type is which is sent by the device on the MIDI interface when the corresponding button
  or pad is pressed.
  11
  Factory default Lighting Custom Mode layouts
  Custom Mode 3:
  Custom Mode 4:
  12
  Colour palette
  When providing colours by MIDI notes or control changes, the colours are chosen according to the
  following table, decimal:
  The same table with hexadecimal indexing:
  13
  Sending colours by MIDI events
  In Lighting Custom Modes (and Programmer mode) the Launchpad X accepts colours through the MIDI
  interface. Channels are used as follows:
- Channel 1, Notes: 90h (144), Control Changes: B0h (176): Static colour.
- Channel 2, Notes: 91h (145), Control Changes: B1h (177): Flashing colour.
- Channel 3, Notes: 92h (146), Control Changes: B2h (178): Pulsing colour.
  The Note number selects the pad to control (as shown on the layout mappings) and the velocity the
  colour to use (as shown on the colour palette). Similarly, for Control Changes the CC number selects
  the pad to control and the value the colour to use.
  Flashing colour
  When sending Flashing colour, the colour flashes between that set as Static or Pulsing colour (A), and
  that contained in the MIDI event setting flashing (B), at 50% duty cycle, synchronized to the MIDI beat
  clock (or 120bpm or the last clock if no clock is provided). One period is one beat long.
  Pulsing colour
  The colour pulses between dark and full intensity synchronized to the MIDI beat clock (or 120bpm or
  the last clock if no clock is provided). One period is two beats long, using the following waveform:
  14
  Examples
  For these examples, switch the Launchpad into Programmer Mode.
  Lighting the lower left pad static red:
  Host => Launchpad X:
  Hex: 90h 0Bh 05h
  Dec: 144 11 5
  This is Note On, Channel 1, Note number 0Bh (11), with Velocity 05h (5). The Channel specifies the
  lighting mode (static), the Note number the pad to light (which is the lower left one in Programmer
  mode), the Velocity the colour (which is Red, see Colour Palette).
  Flashing the upper left pad green:
  Host => Launchpad X:
  Hex: 91h 51h 13h
  Dec: 145 81 19
  This is Note On, Channel 2, Note number 51h (81), with Velocity 13h (19). The Channel specifies the
  lighting mode (flashing), the Note number the pad to light (which is the upper left one in Programmer
  mode), the Velocity the colour (which is Green, see Colour Palette).
  Pulsing the lower right pad blue:
  Host => Launchpad X:
  Hex: 92h 12h 2Dh
  Dec: 146 18 45
  This is Note On, Channel 3, Note number 12h (18), with Velocity 2Dh (45). The Channel specifies the
  lighting mode (pulsing), the Note number the pad to light (which is the lower right one in Programmer
  mode), the Velocity the colour (which is Blue, see Colour Palette).
  Turning a colour off:
  Host => Launchpad X:
  Hex: 90h 12h 00h
  Dec: 144 18 0
  This is Note Off (Note On with Velocity of zero), Channel 1, Note number 12h (18), with Velocity 00h
  (0). The Channel specifies the lighting mode (static), the Note number the pad to light (which is the
  lower right one in Programmer mode), the Velocity the colour (which is blank, see Colour Palette). If
  the Pulsing colour was set up there with the previous message, this would turn it off. Alternatively, a
  Midi Note Off message can also be used for the same effect:
  Host => Launchpad X:
  Hex: 80h 12h 00h
  Dec: 128 18 0
  15
  LED lighting SysEx message
  This message can be sent to Lighting Custom Modes and the Programmer mode to light up LEDs. The
  LED indices used always correspond to those of Programmer mode, regardless of the layout selected:
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch 03h <colourspec> [<colourspec> […]] F7h
  Dec: 240 0 32 41 2 12 3 <colourspec> [<colourspec> […]] 247
  The <colourspec> is structured as follows:
- Lighting type (1 byte)
- LED index (1 byte)
- Lighting data (1 – 3 bytes)
  Lighting types:
- 0: Static colour from palette, Lighting data is 1 byte specifying palette entry.
- 1: Flashing colour, Lighting data is 2 bytes specifying Colour B and Colour A.
- 2: Pulsing colour, Lighting data is 1 byte specifying palette entry.
- 3: RGB colour, Lighting data is 3 bytes for Red, Green and Blue (127: Max, 0: Min).
  The message may contain up to 81 <colourspec> entries to light up the entire Launchpad X surface.
  Example:
  Host => Launchpad X:
  Hex: F0h 00h 20h 29h 02h 0Ch 03h 00h 0Bh 0Dh 01h 0Ch 15h 17h 02h 0Dh 25h F7h
  Dec: 240 0 32 41 2 12 3 0 11 13 1 12 21 23 2 13 37 247
  Sending this message to the Launchpad X in Programmer layout sets up the bottom left pad to static
  yellow, the pad next to it to flashing green (between dim and bright green), and the pad next to that
  pulsing turquoise.