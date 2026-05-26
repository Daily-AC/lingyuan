use world::World;

pub fn serialize_world(w: &World) -> anyhow::Result<Vec<u8>> {
    Ok(bincode::serialize(w)?)
}

pub fn deserialize_world(bytes: &[u8]) -> anyhow::Result<World> {
    Ok(bincode::deserialize(bytes)?)
}
