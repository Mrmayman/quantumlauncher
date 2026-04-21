use crate::store::StoreBackendType;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModId {
    Modrinth(String),
    Curseforge(String),
}

impl serde::Serialize for ModId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ModId::Modrinth(id) => serializer.serialize_str(id),
            ModId::Curseforge(id) => serializer.serialize_str(&format!("CF:{id}")),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ModId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ModIdVisitor;
        impl<'de> serde::de::Visitor<'de> for ModIdVisitor {
            type Value = ModId;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a string representing a mod id")
            }
            fn visit_str<E>(self, value: &str) -> Result<ModId, E>
            where
                E: serde::de::Error,
            {
                if let Some(rest) = value.strip_prefix("CF:") {
                    Ok(ModId::Curseforge(rest.to_string()))
                } else {
                    Ok(ModId::Modrinth(value.to_string()))
                }
            }
        }
        deserializer.deserialize_str(ModIdVisitor)
    }
}

impl ModId {
    #[must_use]
    pub fn get_internal_id(&self) -> &str {
        match self {
            ModId::Modrinth(n) | ModId::Curseforge(n) => n,
        }
    }

    #[must_use]
    pub fn get_backend(&self) -> StoreBackendType {
        match self {
            ModId::Modrinth(_) => StoreBackendType::Modrinth,
            ModId::Curseforge(_) => StoreBackendType::Curseforge,
        }
    }

    #[must_use]
    pub fn from_pair(n: &str, t: StoreBackendType) -> Self {
        let n = n.to_owned();
        match t {
            StoreBackendType::Modrinth => Self::Modrinth(n),
            StoreBackendType::Curseforge => Self::Curseforge(n),
        }
    }

    #[must_use]
    pub fn to_pair(self) -> (String, StoreBackendType) {
        let backend = match self {
            ModId::Modrinth(_) => StoreBackendType::Modrinth,
            ModId::Curseforge(_) => StoreBackendType::Curseforge,
        };

        (self.get_internal_id().to_owned(), backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_modrinth() {
        let id = ModId::Modrinth("abc123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"abc123\"");
    }

    #[test]
    fn serialize_curseforge() {
        let id = ModId::Curseforge("1074338".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"CF:1074338\"");
    }

    #[test]
    fn deserialize_modrinth() {
        let json = "\"abc123\"";
        let id: ModId = serde_json::from_str(json).unwrap();
        assert_eq!(id, ModId::Modrinth("abc123".to_string()));
    }

    #[test]
    fn deserialize_curseforge() {
        let json = "\"CF:1074338\"";
        let id: ModId = serde_json::from_str(json).unwrap();
        assert_eq!(id, ModId::Curseforge("1074338".to_string()));
    }

    #[test]
    fn roundtrip_modrinth() {
        let original = ModId::Modrinth("xyz789".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ModId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn roundtrip_curseforge() {
        let original = ModId::Curseforge("555".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ModId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn deserialize_cf_empty_suffix() {
        let json = "\"CF:\"";
        let id: ModId = serde_json::from_str(json).unwrap();
        assert_eq!(id, ModId::Curseforge("".to_string()));
    }

    #[test]
    fn deserialize_non_cf_prefix() {
        let json = "\"CFA:123\"";
        let id: ModId = serde_json::from_str(json).unwrap();
        assert_eq!(id, ModId::Modrinth("CFA:123".to_string()));
    }

    #[test]
    fn hashmap_key_serialize_deserialize() {
        use serde_json;
        use std::collections::HashMap;

        let mut map: HashMap<ModId, i32> = HashMap::new();
        map.insert(ModId::Modrinth("abc123".to_string()), 1);
        map.insert(ModId::Curseforge("1074338".to_string()), 2);

        let json = serde_json::to_string(&map).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["abc123"], 1);
        assert_eq!(value["CF:1074338"], 2);

        let parsed: HashMap<ModId, i32> = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get(&ModId::Modrinth("abc123".to_string())), Some(&1));
        assert_eq!(
            parsed.get(&ModId::Curseforge("1074338".to_string())),
            Some(&2)
        );
    }
}
