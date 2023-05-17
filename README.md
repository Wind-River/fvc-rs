# File Verification Code

File Verification Code is a tool to find whether two given packages/archives are equivalent, i.e, the files inside the two should be exactly same. It traverses the entire package/archive and calculates the SHA256 of each file. These are added to a list which is sorted and finally the SHA256 of this list is calculated. Any package/archive containing exactly the same files will have the same verification code no matter the structure or name of the package/archive. File verification code is useful as an identifier or unique id for a given package/archive.

By default fvc is compiled with the [extract](#extract) feature, requiring [libarchive](https://www.libarchive.org/) to be installed and findable.
To build without this dependency use the cli option `--no-default-features`.

## Features
### extract
The extract feature enables use of libarchive to extract any given or encountered archives, and then processes their contents.
If this is disabled, any archive is treated as a file.