#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>

void archiveData(id dict, NSString* name) {
    NSError* arcError = nil;
    NSData* data = [NSKeyedArchiver
     archivedDataWithRootObject: dict
     requiringSecureCoding: NO
     error: &arcError
    ];
    if(arcError != nil) {
        NSLog(@"Archiver error: %@", [arcError localizedDescription]);
    }

    NSError *writingError = nil;
    NSString* path= [NSString stringWithFormat: @"./plists/%@.plist", name];
    [data writeToFile:path options:NSDataWritingAtomic error:&writingError];
    if(writingError != nil) {
        NSLog(@"Writing error: %@", [writingError localizedDescription]);
    }
}

void archiveNSAffineTransform(void) {
    NSAffineTransform* a = [[NSAffineTransform alloc] init];
    [a rotateByDegrees: 15.5];
    archiveData(a, @"NSAffineTransform");
}

void plainString(void) {
    NSString* str = @"Some string!";
    archiveData(str,  @"plainString");
}

void nsData(void) {
    NSString* str = @"Some data!";
    NSData* data = [str dataUsingEncoding:NSUTF8StringEncoding];
    archiveData(data,  @"nsData");
}

void simpleArray(void) {
    NSArray *innerArray = [NSArray arrayWithObjects:
        @"innerValue3",
        @"innerValue4",
        nil];
    NSArray *myArray = [NSArray arrayWithObjects:
        @"value1",
        @"value2",
        innerArray, nil];
    archiveData(myArray, @"simpleArray");
}

void simpleDict(void) {
    NSDictionary *dict = @{
        @"First key":@"First value",
        @"Second key":@"Second value",
        @"Array key":@[@1,@2,@3]
    };
    archiveData(dict, @"simpleDict");
}

void circularReference(void) {
    NSMutableArray *array = [NSMutableArray arrayWithObjects:
    @"self reference here",
    nil];
    array[1] = array;
    archiveData(array, @"circularReference");
}

int main(int argc, const char * argv[]) {
    @autoreleasepool {
        plainString();
        nsData();
        simpleArray();
        simpleDict();
        circularReference();
    }
    return 0;
}
