use fs_extra::dir::copy;
use std::path::Path;
use std::fs;
use ql_core::file_utils::get_launcher_dir;
use walkdir::WalkDir;


pub async fn clone_all(instance_type : String, instance_name: String ,  new_instance_name: String) {

            let from = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join(&instance_type).join(instance_name);
            let to = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join(&instance_type).join(new_instance_name);

            fs::create_dir_all(&to).expect("Unable to create directory!");
            copy(from, to, &Default::default()).expect("Unable to copy!");
}


pub async fn clone_selected(instance_type : String, instance_name: String, new_instance_name: String, selected: Vec<String>)  {

            let from = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join(&instance_type).join(instance_name);
            let to = Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join(&instance_type).join(new_instance_name);

            for entry in WalkDir::new(&from) {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy();

                if selected.contains(&name.to_string()) {
                    let relative_path = entry.path().strip_prefix(&from).unwrap();
                    let target = to.join(relative_path);

                    if entry.file_type().is_dir() {
                        fs::create_dir_all(&target).unwrap();
                    }
                    else {
                        fs::create_dir_all(target.parent().unwrap()).unwrap();
                        fs::copy(entry.path(), &target).unwrap();
                    }
                }
            }
}


// For fronted integration

async fn clone() {

    let current_instance_name = String::from("SELECT ME THE CURRENT SERVER NAME HERE"); // TODO NAME MUST BE FOLDER NAME OF THE SELECTED INSTANCE/SERVER

    let new_instance_name ;
    // TODO GET USER INPUT and input the name for the new server
    new_instance_name = "placeholder";  // PLACEHOLDER TODO REMOVE WHEN IMPLEMENTED INPUT
    let trimmed_name = String::from(new_instance_name.trim());

    let is_server = false;
    // TODO MAKE THIS OPTION GET INPUT BY THE TYPE OF SELECTED INSTANCE!!!!!!

    let clone_all_option = false;
    // TODO MAKE THIS OPTION GET INPUT BY THE USER IF SELECTED CLONING IS SELECTED THIS SHOULD = FALSE!

    let instance_type ;
    if is_server == false {instance_type = String::from("instance") }
    else {instance_type = String::from("server")};

    if Path::new(get_launcher_dir().unwrap().as_path()).join("QuantumLauncher").join(&instance_type).join(&new_instance_name).is_dir() == true {
        println!("Directory already exists! Try a different name.")
    }
    else {

        if clone_all_option == true{
            clone_all(instance_type, current_instance_name, trimmed_name).await;
            println!("Cloned profile successfully")
        }
        else {
            let selected: Vec<String>;
            // TODO INPUT SELECTED FOLDERS TO THIS VECTOR like this -> "folder/".to_string(), "file.txt.to_string()" MUST USE .to_string() bc IF I INPUT JUST "example" IT IS &str NOT STRING
            selected = vec!["testing".to_string(), "asd".to_string()]; // PLACEHOLDER TODO REMOVE WHEN IMPLEMENTED INPUT

            clone_selected(instance_type, current_instance_name, trimmed_name, selected).await;
        }
    }

}