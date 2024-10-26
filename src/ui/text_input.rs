#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextInput {
    ArtnetOutputDefaultIp,
    ArtnetOutputIp(u16),
    ArtnetOutputUniverse(u16),
    WledDrgbOutputDefaultIp,
    WledDrgbOutputDefaultPort,
    WledDrgbOutputIp(u16),
    WledDrgbOutputPort(u16),
    WledDnrgbOutputDefaultIp,
    WledDnrgbOutputDefaultPort,
    WledDnrgbOutputDefaultStart,
    WledDnrgbOutputIp(u16),
    WledDnrgbOutputPort(u16),
    WledDnrgbOutputStart(u16),
}
