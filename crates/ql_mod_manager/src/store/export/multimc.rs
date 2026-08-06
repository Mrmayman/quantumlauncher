use serde::Serialize;
use ql_core::{Instance, json::VersionDetails};
use crate::store::export::{
        package_multimc_modpack,
        ModpackExportError
};
use std::path::{PathBuf, Path};
#[derive(Serialize)]
pub struct MmcPack {
    #[serde(rename = "formatVersion")]
    pub format_version: i16,
    pub components: Vec<Components>,
}

#[derive(Serialize)]
pub struct Components {
    pub uid: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct InstanceCfg {
    pub instance_type: String,
    pub name: String,
    pub notes: String,
    pub jvm_args: Option<String>,
    pub max_mem_alloc: Option<String>,
}

pub async fn export_multimc_modpack(
    modpack_path: String,
    modpack_name: String,
    modpack_summary: String,
    modpack_file_name: String,
    content: Vec<String>, // MUST BE FULL PATH!! | This is all files including mods and resources packs ect.!!
    instance: Instance,
    jvm_args: Option<String>,
    max_mem_alloc: Option<String>,
    lwjgl: Option<String>, // Must be [[ (for lwjgl 2)-> org.lwjgl || org.lwjgl3 ]] or None!!
    lwjgl_version: Option<String> // If lwjgl is not None this must contain a version!!
) -> Result<(), ModpackExportError> {

    let details = VersionDetails::load(&instance).await?;
    let minecraft_version = details.get_id();

    let config = ql_core::InstanceConfigJson::read(&instance).await?;
    let loader_id = match config.mod_type.to_modrinth_str() {
        "fabric" => "net.fabricmc.fabric-loader".to_string(),
        "quilt" => "org.quiltmc.quilt-loader".to_string(),
        "forge" => "net.minecraftforge".to_string(),
        "neoforge" => "net.neoforged".to_string(),
        "liteloader" => "com.mumfrey.liteloader".to_string(),
        _ => panic!("Unsupported loader type"),
    };
    let loader_version = config.mod_type_info.unwrap().version.unwrap().to_string();
    let instance_type;

    if version_type(minecraft_version) == true {
        instance_type = "Legacy";
    } else {
        instance_type = "OneSix";
    }

    let mmc_pack = create_mmc_pack(minecraft_version.to_string(), loader_id, loader_version, lwjgl, lwjgl_version)?;
    let instance_cfg = create_instance_cfg(instance_type.to_string(), modpack_name, modpack_summary, jvm_args, max_mem_alloc);

    let zip_path = PathBuf::from(&modpack_path)
        .join(format!("{}.zip", modpack_file_name))
        .to_string_lossy()
        .to_string();



    let content: Vec<(String, String)> = content
        .into_iter()
        .map(|full| {
            let path = Path::new(&full);
            let relative = path
                .strip_prefix(Path::new(
                    &instance.get_instance_path().to_str().unwrap(),
                ))
                .unwrap_or(path);
            (full.clone(), relative.to_string_lossy().into())
        })
        .collect();


    package_multimc_modpack(mmc_pack, instance_cfg, zip_path, content)
        .await
        ?;

    Ok(())
}

fn create_mmc_pack(
    minecraft_version: String,
    loader_id: String,
    loader_version: String,
    lwjgl: Option<String>, // Must be [[ (for lwjgl 2)-> org.lwjgl || org.lwjgl3 ]] or None!!
    lwjgl_version: Option<String> // If lwjgl is not None this must contain a version!!
) -> Result<String, serde_json::Error> {

    let mut components = Vec::new();

    components.push(Components {
        uid: "net.minecraft".to_string(),
        version: minecraft_version,
    });

    components.push(Components {
        uid: loader_id,
        version: loader_version,
    });

    if let Some(lwjgl_id) = lwjgl {
        components.push(Components {
            uid: lwjgl_id,
            version: lwjgl_version.unwrap(),
        });
    }

    let mmc_pack = MmcPack {
        format_version: 1,
        components,
    };

    let json_data = serde_json::to_string_pretty(&mmc_pack)?;

    Ok(json_data)
}

fn create_instance_cfg(
    instance_type: String,
    modpack_name: String,
    summary: String,
    jvm_args: Option<String>,
    max_mem_alloc: Option<String>
) -> String {

    let mut instance_cfg = format!(
        "InstanceType={}\nname={}\nnotes={}\n",
        instance_type, modpack_name, summary
    );

    if let Some(args) = jvm_args {
        instance_cfg.push_str(&format!("JvmArgs={}\n", args));
    }
    if let Some(mem) = max_mem_alloc {
        instance_cfg.push_str(&format!("MaxMemAlloc={}\n", mem));
    }

    instance_cfg
}


fn version_type(version_id: &str) -> bool { //checks if version is below 1.6
    let version_id = version_id.trim();

    if version_id.starts_with("pc-")
        || version_id.starts_with("rd-")
        || version_id.starts_with("c0.")
        || version_id.starts_with("c.")
        || version_id.starts_with("indev")
        || version_id.starts_with("inf-")
        || version_id.starts_with("inf_")
        || version_id.starts_with("a1.")
        || version_id.starts_with("a0.")
        || version_id.starts_with("b1.")
        || version_id.starts_with("b.")
        || version_id == "old_alpha"
        || version_id == "old_beta"
    {
        return true;
    }

    //snapshots
    if let Some(w_pos) = version_id.find('w') {
        if w_pos == 2 {
            let first_num = &version_id[0..2];
            if let Ok(f_num) = first_num.parse::<u32>() {
                if f_num < 13 { return true; }
                if f_num > 13 { return false; }
                let after_w = &version_id[w_pos + 1..];
                let second_num: String = after_w.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(s_num) = second_num.parse::<u32>() {
                    return s_num < 24;
                }
                return true;
            }
        }
    }

    // numerical releses
    let mut parts = version_id.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    let minor:Option<u32> = parts.next().and_then(|s| {
        s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()
    });

    match (major, minor) {
        (Some(1), Some(m)) => m < 6,
        (Some(m), _) => m < 1,
        _ => true,
    }
}