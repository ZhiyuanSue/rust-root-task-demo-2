use alloc::alloc::alloc_zeroed;
use alloc::vec::{self, Vec};
use sel4::init_thread::SlotRegion;
use sel4::cap_type;
use core::alloc::Layout;
use core::arch::asm;
use core::borrow::BorrowMut;
use core::ops::Range;
use core::slice::SliceIndex;
use spin::Mutex;
use sel4::{CNodeCapData, init_thread::Slot, Cap, UntypedDesc};
use sel4::cap_type::Untyped;
use sel4::ObjectBlueprintArch;
use sel4::UserContext;
use sel4::sys::{seL4_EndpointBits, seL4_PageBits, seL4_TCBBits};
use sel4_root_task::debug_println;
use crate::image_utils::UserImageUtils;


pub static GLOBAL_OBJ_ALLOCATOR: Mutex<ObjectAllocator> = Mutex::new(ObjectAllocator::default());


#[derive(Clone)]
struct UsedUntypedDesc {
    pub desc: UntypedDesc,
    pub used: bool,
}

pub struct ObjectAllocator {
    untyped_list: Vec<UsedUntypedDesc>,
    untyped_start: Slot,
    empty: SlotRegion<cap_type::Null>,
}

#[warn(dead_code)]
impl ObjectAllocator {
    pub fn new(bootinfo: &sel4::BootInfo) -> Self {
        let mut untyped_list = Vec::new();
        let v = bootinfo.untyped_list().to_vec();
        for item in v {
            untyped_list.push(UsedUntypedDesc{
                desc: item,
                used: false,
            })
        }
        Self {
            untyped_list,
            untyped_start: Slot::from_index(bootinfo.untyped().start()),
            empty: bootinfo.empty()
        }
    }

    pub const fn default() -> Self {
        Self {
            untyped_list: Vec::new(),
            untyped_start: Slot::from_index(0),
            empty: SlotRegion::from_range(Range { start: 0, end: 0}),
        }
    }

    pub fn init(&mut self, bootinfo: &sel4::BootInfo) {
        // debug_println!("untyped list: {:?}", bootinfo.untyped_list().to_vec());
        let mut untyped_list = Vec::new();
        let v = bootinfo.untyped_list().to_vec();
        for item in v {
            untyped_list.push(UsedUntypedDesc{
                desc: item,
                used: false,
            })
        }
        self.untyped_list = untyped_list;
        self.untyped_start = Slot::from_index(bootinfo.untyped().start());
        self.empty = bootinfo.empty();
    }

    pub fn get_the_first_untyped_slot(&mut self, blueprint: &sel4::ObjectBlueprint) -> Cap<Untyped> {
        {
            let idx = self
                .untyped_list
                .iter()
                .position(|desc| {
                    !desc.desc.is_device() && desc.desc.size_bits() >= blueprint.physical_size_bits() && desc.used == false
                }).unwrap();
            // debug_println!("blueprint.physical_size_bits(): {:?}, {:?}", 
            //     blueprint.physical_size_bits(),
            //     self.untyped_list[idx].size_bits());
            // self.untyped_list.remove(idx);
            self.untyped_list[idx].used = true;
            let slot = Slot::from_index(self.untyped_start.index() + idx);
            slot.cap()
			// sel4::BootInfo::init_cspace_local_cptr::<Untyped>(slot)
        }
    }

    #[inline]
    pub fn get_empty_slot(&mut self) -> Slot {
        Slot::from_index(self.empty.range().next().unwrap())
    }

    pub fn alloc_ntfn(&mut self) -> sel4::Result<Cap<sel4::cap_type::Notification>> {
        let blueprint = sel4::ObjectBlueprint::Notification;
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot: Slot<sel4::cap_type::Notification> = Slot::from_index(self.empty.range().next().unwrap());
        let cnode = sel4::init_thread::slot::CNODE.cap();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot.index(),
            1,
        )?;
        Ok(slot.cap())
		// Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        //     slot,
        // ))
    }

    pub fn alloc_ep(&mut self) -> sel4::Result<Cap<sel4::cap_type::Endpoint>> {
        let blueprint = sel4::ObjectBlueprint::Endpoint;
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot: Slot<sel4::cap_type::Endpoint> = Slot::from_index(self.empty.range().next().unwrap());
        let cnode = sel4::init_thread::slot::CNODE.cap();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot.index(),
            1,
        )?;
        Ok(slot.cap())
		// Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        //     slot,
        // ))
    }

    pub fn alloc_many_ep(&mut self, cnt_bits: usize) -> Vec<Cap<sel4::cap_type::Endpoint>> {
        let cnt = 1 << cnt_bits;
        let mut ans = Vec::with_capacity(cnt);
        let blueprint = sel4::ObjectBlueprint::Untyped {
            size_bits: seL4_EndpointBits as usize + cnt_bits 
        };
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.range().next().unwrap();
        
        for _ in 1..cnt {
            self.empty.range().next().unwrap();
        }
        let cnode = sel4::init_thread::slot::CNODE.cap();
        let ep_blueprint = sel4::ObjectBlueprint::Endpoint;
        untyped.untyped_retype(
            &ep_blueprint,
            &cnode.relative_self(),
            slot,
            cnt
        ).unwrap();
        for i in 0..cnt {
            ans.push(sel4::init_thread::Slot::from_index(i).cap())
			// ans.push(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Endpoint>(slot + i))
        };
        return ans;
    }

    pub fn alloc_frame(&mut self) -> sel4::Result<Cap<sel4::cap_type::_4kPage>> {
        let blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::_4kPage);
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot: Slot<sel4::cap_type::_4kPage> = Slot::from_index(self.empty.range().next().unwrap());
        let cnode = sel4::init_thread::slot::CNODE.cap();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot.index(),
            1,
        )?;
        Ok(slot.cap())
		// Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        //     slot,
        // ))
    }

    pub fn alloc_many_frame(&mut self, cnt_bits: usize) -> Vec<Cap<sel4::cap_type::_4kPage>> {
        let cnt = 1 << cnt_bits;
        let mut ans = Vec::with_capacity(cnt);
        let blueprint = sel4::ObjectBlueprint::Untyped {
            size_bits: seL4_PageBits as usize + cnt_bits 
        };
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.range().next().unwrap();
        
        for _ in 1..cnt {
            self.empty.range().next().unwrap();
        }
        let cnode: Cap<sel4::cap_type::CNode> = sel4::init_thread::slot::CNODE.cap();
        let frame_blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::_4kPage);
        untyped.untyped_retype(
            &frame_blueprint,
            &cnode.relative_self(),
            slot,
            cnt
        ).unwrap();
        for i in 0..cnt {
            ans.push(sel4::init_thread::Slot::from_index(i).cap())
			// ans.push(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::_4kPage>(slot + i))
        };
        return ans;
    }

    pub fn alloc_tcb(&mut self) -> sel4::Result<Cap<sel4::cap_type::Tcb>> {
        let blueprint = sel4::ObjectBlueprint::Tcb;
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot: Slot<sel4::cap_type::Tcb> = Slot::from_index(self.empty.range().next().unwrap());
        let cnode = sel4::init_thread::slot::CNODE.cap();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot.index(),
            1,
        )?;
        Ok(slot.cap())
		// Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        //     slot,
        // ))
    }

    pub fn alloc_many_tcb(&mut self, cnt_bits: usize) -> Vec<Cap<sel4::cap_type::Tcb>> {
        let cnt = 1 << cnt_bits;
        let mut ans = Vec::with_capacity(cnt);
        let blueprint = sel4::ObjectBlueprint::Untyped {
            size_bits: seL4_TCBBits as usize + cnt_bits 
        };
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot = self.empty.range().next().unwrap();
        
        for _ in 1..cnt {
            self.empty.range().next().unwrap();
        }
        let cnode = sel4::init_thread::slot::CNODE.cap();
        let tcb_blueprint = sel4::ObjectBlueprint::Tcb;
        untyped.untyped_retype(
            &tcb_blueprint,
            &cnode.relative_self(),
            slot,
            cnt
        ).unwrap();
        for i in 0..cnt {
            ans.push(sel4::init_thread::Slot::from_index(i).cap())
			// ans.push(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Tcb>(slot + i))
        };
        return ans;
    }

    pub fn alloc_page_table(&mut self) -> sel4::Result<Cap<sel4::cap_type::PageTable>> {
        let blueprint = sel4::ObjectBlueprint::Arch(ObjectBlueprintArch::PageTable);
        let untyped = self.get_the_first_untyped_slot(&blueprint);
        let slot: Slot<sel4::cap_type::PageTable> = Slot::from_index(self.empty.range().next().unwrap());
        let cnode = sel4::init_thread::slot::CNODE.cap();
        untyped.untyped_retype(
            &blueprint,
            &cnode.relative_self(),
            slot.index(),
            1,
        )?;
        Ok(slot.cap())
		// Ok(sel4::BootInfo::init_cspace_local_cptr::<sel4::cap_type::Notification>(
        //     slot,
        // ))
    }

    pub fn create_many_threads(&mut self, cnt_bits: usize, func: fn(usize, usize), args: Vec<usize>, prio: usize, affinity: u64, resume: bool) -> Vec<Cap<sel4::cap_type::Tcb>> {
        let cnt = 1 << cnt_bits;
        assert_eq!(args.len(), cnt);
        // debug_println!("untypedlist info: {:?}", self.untyped_list);
        let eps = self.alloc_many_ep(cnt_bits);
        let tcbs = self.alloc_many_tcb(cnt_bits);
        let cnode = sel4::init_thread::slot::CNODE.cap();
        let vspace = sel4::init_thread::slot::VSPACE.cap();
        for i in 0..cnt {
            let ipc_buffer_layout = Layout::from_size_align(4096, 4096)
            .expect("Failed to create layout for page aligned memory allocation");
            let ipc_buffer_addr = unsafe {
                let ptr = alloc_zeroed(ipc_buffer_layout);
                if ptr.is_null() {
                    panic!("Failed to allocate page aligned memory");
                }
                ptr as usize
            };
            let ipc_buffer_cap = UserImageUtils.get_user_image_frame_slot(ipc_buffer_addr).index() as u64;
            let tcb = tcbs[i];
            let ep = eps[i];
            let ipc_buffer = Cap::<sel4::cap_type::_4kPage>::from_bits(ipc_buffer_cap);
            tcb.tcb_configure(ep.cptr(), cnode, CNodeCapData::new(0, 0), vspace, ipc_buffer_addr as u64, ipc_buffer).unwrap();
            tcb.tcb_set_sched_params(sel4::init_thread::slot::TCB.cap(), prio as u64, prio as u64).unwrap();
            let mut user_context = tcb.tcb_read_registers(false, (core::mem::size_of::<UserContext>() / sel4::WORD_SIZE) as u64).unwrap();

            let new_stack_layout = Layout::from_size_align(4096 * 64, 4096).expect("Failed to create layout for page aligned memory allocation");
            let raw_sp = unsafe {
                let ptr = alloc_zeroed(new_stack_layout);
                if ptr.is_null() {
                    panic!("Failed to allocate page aligned memory");
                }
                ptr.add(4096 * 64) as u64
            };
            let mut tp = raw_sp - 4096 * 32;
            tp = tp & (!((1 << 12) - 1));
            debug_println!("tp: {:#x}", tp);

            user_context.inner_mut().tp = tp;
            *(user_context.pc_mut()) = unsafe { core::mem::transmute(func) };
            *(user_context.sp_mut()) = tp & !(16 - 1);

            user_context.inner_mut().s0 = 0;
            user_context.inner_mut().s1 = 0;


            let gp: u64;
            unsafe {
                asm!("mv {}, gp", out(reg) gp);
            }
            user_context.inner_mut().gp = gp;
            user_context.inner_mut().a0 = args[i] as u64;
            user_context.inner_mut().a1 = ipc_buffer_addr as u64;
            debug_println!("write register: {:?}", user_context);
            tcb.tcb_write_all_registers(false, &mut user_context).unwrap();

            // tcb.tcb_set_affinity(affinity).unwrap();
            if resume {
                tcb.tcb_resume().unwrap();
            }
        }
        tcbs
    }

    pub fn create_thread(&mut self, func: fn(usize, usize), args: usize, prio: usize, affinity: u64, resume: bool) -> sel4::Result<Cap<sel4::cap_type::Tcb>>
    {
        let ipc_buffer_layout = Layout::from_size_align(4096, 4096)
            .expect("Failed to create layout for page aligned memory allocation");
        let ipc_buffer_addr = unsafe {
            let ptr = alloc_zeroed(ipc_buffer_layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr as usize
        };
        let ipc_buffer_cap = UserImageUtils.get_user_image_frame_slot(ipc_buffer_addr).index() as u64;
        let tcb = self.alloc_tcb()?;
        let ep = self.alloc_ep()?;
        let cnode = sel4::init_thread::slot::CNODE.cap();
        let vspace = sel4::init_thread::slot::VSPACE.cap();
        let ipc_buffer = Cap::<sel4::cap_type::_4kPage>::from_bits(ipc_buffer_cap);
        tcb.tcb_configure(ep.cptr(), cnode, CNodeCapData::new(0, 0), vspace, ipc_buffer_addr as u64, ipc_buffer)?;
        tcb.tcb_set_sched_params(sel4::init_thread::slot::TCB.cap(), prio as u64, prio as u64)?;
        let mut user_context = tcb.tcb_read_registers(false, (core::mem::size_of::<UserContext>() / sel4::WORD_SIZE) as u64)?;

        let new_stack_layout = Layout::from_size_align(4096 * 256, 4096).expect("Failed to create layout for page aligned memory allocation");
        let raw_sp = unsafe {
            let ptr = alloc_zeroed(new_stack_layout);
            if ptr.is_null() {
                panic!("Failed to allocate page aligned memory");
            }
            ptr.add(4096 * 256) as u64
        };
        let mut tp = raw_sp - 4096 * 128;
        tp = tp & (!((1 << 12) - 1));
        // debug_println!("tp: {:#x}", tp);

        user_context.inner_mut().tp = tp;
        *(user_context.pc_mut()) = unsafe { core::mem::transmute(func) };
        *(user_context.sp_mut()) = tp & !(16 - 1);

        user_context.inner_mut().s0 = 0;
        user_context.inner_mut().s1 = 0;


        let gp: u64;
        unsafe {
            asm!("mv {}, gp", out(reg) gp);
        }
        user_context.inner_mut().gp = gp;
        user_context.inner_mut().a0 = args as u64;
        user_context.inner_mut().a1 = ipc_buffer_addr as u64;
        // debug_println!("write register: {:?}", user_context);
        tcb.tcb_write_all_registers(false, &mut user_context)?;

        // tcb.tcb_set_affinity(affinity)?;
        if resume {
            tcb.tcb_resume()?;
        }
        Ok(tcb)
    }
}