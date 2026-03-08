use fs_extra::dir::copy;
use std::path::Path;
use std::fs;
use ql_core::file_utils::get_launcher_dir;


pub async fn all_clone_instance(new_instance_type : String, clone_from: String, new_instance_name: String) {
    match new_instance_type.as_str() {
        "instance" => {
            let from = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join("instances").join(clone_from);
            let to = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join("instances").join(new_instance_name);
            fs::create_dir_all(&to).expect("Unable to create directory!");
            copy(from, to, &Default::default()).expect("Unable to copy!");
        },
        "server" => {
            let from = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join("servers").join(clone_from);
            let to = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join("servers").join(new_instance_name);
            fs::create_dir_all(&to).expect("Unable to create directory!");
            copy(from, to, &Default::default()).expect("Unable to copy!");
        },
        _ => {}
    }
}






// For fronted integration

// FOR INSTANCE
async fn clone() {

    let current_instance = String::from("SELECT ME THE CURRENT INSTANCE NAME HERE"); // NAME MUST BE FOLDER NAME

    let mut new_instance_name = String::new();
    // GET USER INPUT and input the name for the new instance
    let trimmed_name = String::from(new_instance_name.trim());

    let instance_type:String = String::from("instance");

    if Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join("instances").join(&new_instance_name).is_dir() == true {
        println!("Directory already exists! Try a different name.")
    }
    else {
        all_clone_instance(instance_type, current_instance, trimmed_name).await;
        println!("Cloned profile successfully")
    }

}


//FOR SERVER
async fn server_clone() {

    let current_instance = String::from("SELECT ME THE CURRENT SERVER NAME HERE"); // NAME MUST BE FOLDER NAME

    let mut new_instance_name = String::new();
    // GET USER INPUT and input the name for the new server
    let trimmed_name = String::from(new_instance_name.trim());

    let instance_type:String = String::from("server");

    if Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join("servers").join(&new_instance_name).is_dir() == true {
        println!("Directory already exists! Try a different name.")
    }
    else {
        all_clone_instance(instance_type, current_instance, trimmed_name).await;
        println!("Cloned profile successfully")
    }

}

