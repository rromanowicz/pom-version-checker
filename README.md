# Simple app to check maven dependencies for new versions.
- Supports multi-module projects.
- Resolves version from properties and parent dependencies.

>### Usage
>```
>cargo run [projectGroup] [pathToProject]
>```
>
>```projectGroup``` - part of the local project group, to skip querying maven repository
>
>```pathToProject``` - path to project root directory (absolute or relative)

>### Output
> - Prints only dependencies that have newer versions
> - Example:
>>```
>>"main-project"
>>    {"groupId": "com.foo","artifactId": "artifact-1", "version": "1.0.0", "latestVersion": "1.3.2"}
>>    {"groupId": "com.foo","artifactId": "artifact-2", "version": "1.0.1", "latestVersion": "1.0.2"}
>>"submodule"
>>    {"groupId": "com.bar.","artifactId": "artifact-1", "version": "3.1", "latestVersion": "3.3.2"}
>>    {"groupId": "com.bar","artifactId": "artifact-2", "version": "2.6", "latestVersion": "3.0"}
>>```
